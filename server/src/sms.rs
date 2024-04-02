use rusoto_core::{request::HttpClient, Region, RusotoError};
use rusoto_credential::EnvironmentProvider;
use rusoto_sns::{MessageAttributeValue, PublishError, PublishInput, Sns, SnsClient};

use std::collections::HashMap;
use std::env;

lazy_static! {
    /// SMS receiver number (E.164 format).
    static ref SMS_RECEIVER: Option<String>
        = env::var("SMS_RECEIVER").ok();

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
