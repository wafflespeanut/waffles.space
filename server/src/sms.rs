use http::header;
use reqwest::Client;
use rusoto_core::{request::HttpClient, Region, RusotoError};
use rusoto_credential::EnvironmentProvider;
use rusoto_sns::{MessageAttributeValue, PublishError, PublishInput, Sns, SnsClient};

use std::collections::HashMap;
use std::env;

lazy_static! {
    /// SMS receiver number (E.164 format).
    static ref SMS_RECEIVER: Option<String>
        = env::var("SMS_RECEIVER").ok();

    /* Twilio */
    /// Twilio account ID set in environment.
    static ref TWILIO_ACCOUNT: Option<String>
        = env::var("TWILIO_ACCOUNT").ok();
    /// Twilio access token set in environment.
    static ref TWILIO_TOKEN: Option<String>
        = env::var("TWILIO_TOKEN").ok();
    /// Twilio sender number.
    static ref TWILIO_SENDER: Option<String>
        = env::var("TWILIO_SENDER").ok();
    static ref TWILIO_API_ENDPOINT: Option<String>
        = TWILIO_ACCOUNT.as_ref().and_then(|a| Some(format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", a)));

    /* AWS */

    // Uses `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY`
    static ref AWS_ENV: EnvironmentProvider = EnvironmentProvider::default();
    /// AWS region used by the provider.
    static ref AWS_REGION: Option<Region>
        = env::var("AWS_REGION").ok().and_then(|s| s.parse().ok());
    static ref AWS_CLIENT: Option<SnsClient> = AWS_REGION.as_ref().map(|region| {
        SnsClient::new_with(HttpClient::new().expect("creating https client"), AWS_ENV.clone(), region.clone())
    });

    static ref AWS_MSG_ATTRS: Option<HashMap<String, MessageAttributeValue>> = {
        let mut h = HashMap::new();
        h.insert("AWS.SNS.SMS.SMSType".into(), MessageAttributeValue {
            binary_value: None,
            data_type: "String".into(),
            string_value: Some("Transactional".into()),
        });
        Some(h)
    };

    /* Other */

    static ref HTTP_CLIENT: Client = Client::new();
}

/// Sends the message to the given number.
pub async fn send(message: &str) {
    info!("[MESSAGE]\n{}\n", message);
    if SMS_RECEIVER.is_none() {
        return;
    }

    match send_using_aws(message).await {
        Ok(true) => return,
        Ok(false) => (),
        Err(e) => error!("Error sending message using AWS: {:?}", e),
    }

    match send_using_twilio(message).await {
        Ok(true) => return,
        Ok(false) => (),
        Err(e) => error!("Error sending message using Twilio: {:?}", e),
    }

    error!("No supported SMS providers have been configured.");
}

/// Sends a message using AWS SNS API.
async fn send_using_aws(message: &str) -> Result<bool, RusotoError<PublishError>> {
    info!("Sending message using AWS.");
    let (client, receiver) = match (AWS_CLIENT.as_ref(), SMS_RECEIVER.as_ref()) {
        (Some(c), Some(r)) => (c, r),
        _ => {
            info!("Missing environment variables for AWS API.");
            return Ok(false);
        }
    };

    let resp = client
        .publish(PublishInput {
            message: message.into(),
            phone_number: Some(receiver.clone()),
            message_attributes: AWS_MSG_ATTRS.clone(),
            ..Default::default()
        }).await?;
    info!("AWS response: {:?}", resp);
    Ok(true)
}

/// Send a message using Twilio API.
async fn send_using_twilio(message: &str) -> Result<bool, reqwest::Error> {
    info!("Sending message using Twilio.");
    let (sender, receiver, endpoint, account, token) = match (
        TWILIO_SENDER.as_ref(),
        SMS_RECEIVER.as_ref(),
        TWILIO_API_ENDPOINT.as_ref(),
        TWILIO_ACCOUNT.as_ref(),
        TWILIO_TOKEN.as_ref(),
    ) {
        (Some(s), Some(r), Some(e), Some(a), Some(t)) => (s, r, e, a, t),
        _ => {
            info!("Missing environment variables for Twilio API.");
            return Ok(false);
        }
    };

    let mut params = HashMap::new();
    params.insert("From", sender.as_str());
    params.insert("To", receiver.as_str());
    params.insert("Body", message);

    let response = HTTP_CLIENT
        .post(endpoint)
        .header(
            header::CONTENT_TYPE,
            mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
        )
        .basic_auth(account, Some(token))
        .form(&params)
        .send().await?;
    
    let status = response.status();
    let json: serde_json::Value = response.json().await?;
    info!("{}: {}", status, json);

    Ok(true)
}
