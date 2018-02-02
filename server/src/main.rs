extern crate chrono;
extern crate env_logger;
extern crate iron;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate mount;
extern crate notify;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate staticfile;
extern crate uuid;

mod custom;
mod server;
mod utils;
mod watcher;

fn main() {
    server::start();
}
