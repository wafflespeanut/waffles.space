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

fn main() {
    server::start();
}
