use chrono::offset::Utc;
use env_logger::LogBuilder;
use iron::Iron;
use log::{LogRecord, LogLevelFilter};
use mount::Mount;
use staticfile::Static;
use std::env;

lazy_static! {
    static ref DEFAULT_ADDRESS: String =
        env::var("ADDRESS").unwrap_or(String::from("localhost:8000"));
    static ref SERVE_PATH_ROOT: String =
        env::var("PATH").unwrap_or(String::from("./source"));
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

    info!("Initializing routes...");
    let mut mount = Mount::new();
    mount.mount("/", Static::new(&*SERVE_PATH_ROOT));

    info!("Listening for HTTP requests in {}...", &*DEFAULT_ADDRESS);
    Iron::new(mount).http(&*DEFAULT_ADDRESS).unwrap();
}
