use iron::IronResult;
use iron::error::IronError;
use iron::headers::ContentType;
use iron::middleware::AfterMiddleware;
use iron::request::Request;
use iron::response::Response;
use server::{CUSTOM_4XX, CUSTOM_5XX};

pub struct CustomPage;

impl AfterMiddleware for CustomPage {
    fn catch(&self, _req: &mut Request, err: IronError) -> IronResult<Response> {
        if let Some(s) = err.response.status {
            if s.is_client_error() || s.is_server_error() {
                let body = if s.is_client_error() {
                    CUSTOM_4XX.clone()
                } else {
                    CUSTOM_5XX.clone()
                };

                let mut resp = Response::with((s, body));
                resp.headers.set(ContentType::html());
                return Ok(resp)
            }
        }

        Err(err)
    }
}
