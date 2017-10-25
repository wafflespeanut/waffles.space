use utils;
use chrono::offset::Utc;
use custom::CustomPage;
use env_logger::LogBuilder;
use iron::Iron;
use iron::middleware::Chain;
use log::{LogRecord, LogLevelFilter};
use mount::Mount;
use staticfile::Static;
use std::{env, thread};
use std::fs::File;
use std::io::Read;
use watcher::PrivateWatcher;

lazy_static! {
    static ref DEFAULT_ADDRESS: String =
        env::var("ADDRESS").unwrap_or(String::from("localhost:8000"));
    pub static ref SERVE_PATH_ROOT: String =
        env::var("SOURCE").unwrap_or(String::from("./source"));
    pub static ref PRIVATE_SERVE_PATH: String =
        SERVE_PATH_ROOT.clone() + "/private";
    pub static ref PRIVATE_PATH_ROOT: String =
        env::var("PRIVATE_SOURCE").unwrap_or(String::from("./private"));
    pub static ref CONFIG_FILE: String =
        env::var("CONFIG").unwrap_or(String::from("config.json"));
    pub static ref CUSTOM_4XX: Vec<u8> =
        env::var("CUSTOM_4XX").ok().and_then(|p| {
            File::open(&p).ok().and_then(|mut fd| {
                let mut vec = vec![];
                fd.read_to_end(&mut vec).ok().map(|_| vec)
            })
        }).unwrap_or_else(|| {
            Vec::from(b"Oops! I can't find what you're looking for..." as &[u8])
        });
    pub static ref CUSTOM_5XX: Vec<u8> =
        env::var("CUSTOM_5XX").ok().and_then(|p| {
            File::open(&p).ok().and_then(|mut fd| {
                let mut vec = vec![];
                fd.read_to_end(&mut vec).ok().map(|_| vec)
            })
        }).unwrap_or_else(|| {
            Vec::from(b"Apparently, you've broken me." as &[u8])
        });
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
        let mut watcher = PrivateWatcher::new(&*PRIVATE_PATH_ROOT, &*PRIVATE_SERVE_PATH);
        watcher.start_watching();
    });

    info!("Initializing routes...");
    let mut mount = Mount::new();
    utils::create_dir_if_not_exists(&*SERVE_PATH_ROOT);
    mount.mount("/", Static::new(&*SERVE_PATH_ROOT));

    let _ = (&*CUSTOM_4XX, &*CUSTOM_5XX);
    let mut chain = Chain::new(mount);
    chain.link_after(CustomPage);

    info!("Listening for HTTP requests in {}...", &*DEFAULT_ADDRESS);
    Iron::new(chain).http(&*DEFAULT_ADDRESS).unwrap();
}
