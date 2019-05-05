#![feature(async_await, await_macro)]

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

mod server;
mod staticfile;
mod util;
mod watcher;

fn main() {
    server::start();
}
