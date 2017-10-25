use iron::IronResult;
use iron::error::IronError;
use iron::middleware::AfterMiddleware;
use iron::request::Request;
use iron::response::Response;
use server::{CUSTOM_4XX, CUSTOM_5XX};

pub struct CustomPage;

impl AfterMiddleware for CustomPage {
    fn catch(&self, _req: &mut Request, err: IronError) -> IronResult<Response> {
        if let Some(s) = err.response.status {
            if s.is_client_error() {
                return Ok(Response::with((s, CUSTOM_4XX.clone())))
            } else if s.is_server_error() {
                return Ok(Response::with((s, CUSTOM_5XX.clone())))
            }
        }

        Err(err)
    }
}
