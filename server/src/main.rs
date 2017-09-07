extern crate chrono;
extern crate env_logger;
extern crate iron;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate mount;
extern crate staticfile;

mod server;

fn main() {
    server::start();
}
