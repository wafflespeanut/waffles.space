use utils;
use chrono::offset::Utc;
use env_logger::LogBuilder;
use iron::Iron;
use log::{LogRecord, LogLevelFilter};
use mount::Mount;
use staticfile::Static;
use std::env;
use std::thread;
use watcher::PrivateWatcher;

lazy_static! {
    static ref DEFAULT_ADDRESS: String =
        env::var("ADDRESS").unwrap_or(String::from("localhost:8000"));
    pub static ref SERVE_PATH_ROOT: String =
        env::var("SOURCE").unwrap_or(String::from("./source"));
    pub static ref PRIVATE_PATH_ROOT: String =
        env::var("PRIVATE_SOURCE").unwrap_or(String::from("./private"));
    pub static ref CONFIG_FILE: String =
        env::var("CONFIG").unwrap_or(String::from("config.json"));
}

pub fn start() {
    let mut builder = LogBuilder::new();
    builder.format(|record: &LogRecord| {
        format!("{:?}: {}: {}", Utc::now(), record.level(), record.args())
    }).filter(None, LogLevelFilter::Info);

    if let Ok(v) = env::var("RUST_LOG") {
       builder.parse(&v);
    }

    builder.init().unwrap();
    let _ = thread::spawn(move || {
        let mut watcher = PrivateWatcher::new(&*PRIVATE_PATH_ROOT);
        watcher.start_watching();
    });

    info!("Initializing routes...");
    let mut mount = Mount::new();
    utils::create_dir_if_not_exists(&*SERVE_PATH_ROOT);
    mount.mount("/", Static::new(&*SERVE_PATH_ROOT));

    info!("Listening for HTTP requests in {}...", &*DEFAULT_ADDRESS);
    Iron::new(mount).http(&*DEFAULT_ADDRESS).unwrap();
}
