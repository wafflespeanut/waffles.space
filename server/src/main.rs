#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod server;
mod sms;
mod staticfile;
mod util;
mod watcher;

#[runtime::main(runtime_tokio::Tokio)]
async fn main() {
    server::start().await;
}
