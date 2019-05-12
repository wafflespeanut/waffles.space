use http::header;
use reqwest::Client;

use std::collections::HashMap;
use std::env;

lazy_static! {
    /// Twilio account ID set in environment.
    static ref TWILIO_ACCOUNT: Option<String>
        = env::var("TWILIO_ACCOUNT").ok();
    static ref TWILIO_RECEIVER: Option<String>
        = env::var("TWILIO_RECEIVER").ok();
    /// Twilio access token set in environment.
    static ref TWILIO_TOKEN: Option<String>
        = env::var("TWILIO_TOKEN").ok();
    /// Twilio sender number.
    static ref TWILIO_SENDER: Option<String>
        = env::var("TWILIO_SENDER").ok();
    static ref TWILIO_API_ENDPOINT: Option<String>
        = TWILIO_ACCOUNT.as_ref().and_then(|a| Some(format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", a)));
    // FIXME: I know! Quick cheap way of doing stuff!
    static ref HTTP_CLIENT: Client = Client::new();
}

/// Send a message to the given number using Twilio API.
pub fn send_using_twilio(message: &str) -> Result<(), reqwest::Error> {
    info!("[SENDING MESSAGE]\n{}\n", message);
    let (sender, receiver, endpoint, account, token) = match (
        TWILIO_SENDER.as_ref(),
        TWILIO_RECEIVER.as_ref(),
        TWILIO_API_ENDPOINT.as_ref(),
        TWILIO_ACCOUNT.as_ref(),
        TWILIO_TOKEN.as_ref(),
    ) {
        (Some(s), Some(r), Some(e), Some(a), Some(t)) => (s, r, e, a, t),
        _ => {
            error!("Missing environment variables for Twilio API.");
            return Ok(());
        }
    };

    let mut params = HashMap::new();
    params.insert("From", sender.as_str());
    params.insert("To", receiver.as_str());
    params.insert("Body", message);

    let mut response = HTTP_CLIENT
        .post(endpoint)
        .header(
            header::CONTENT_TYPE,
            mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
        )
        .basic_auth(account, Some(token))
        .form(&params)
        .send()?;

    let json: serde_json::Value = response.json()?;
    info!("{}: {}", response.status(), json);

    Ok(())
}
