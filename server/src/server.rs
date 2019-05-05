use crate::util;
use tide::App;
use crate::staticfile::StaticFile;
use crate::watcher::PrivateWatcher;

use std::{env, thread};
use std::path::Path;

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
}

pub fn start() {
    util::prepare_logger();
    util::create_dir_if_not_exists(&*PRIVATE_PATH_ROOT);
    util::create_dir_if_not_exists(&*SERVE_PATH_ROOT);

    info!("Initializing watcher.");
    let mut watcher = PrivateWatcher::new(&*CONFIG_FILE, &*PRIVATE_PATH_ROOT, &*PRIVATE_SERVE_PATH);
    watcher.initialize();

    let _ = thread::spawn(move || {
        watcher.start_watching();
    });

    info!("Listening for HTTP requests in {}.", &*DEFAULT_ADDRESS);
    let mut app = App::new(());
    let mut static_file = StaticFile::new(&*SERVE_PATH_ROOT);

    let (path_4xx, path_5xx) = (Path::new(&*SERVE_PATH_ROOT).join("4xx.html"),
                                Path::new(&*SERVE_PATH_ROOT).join("5xx.html"));
    if path_4xx.exists() {
        if let Ok(bytes) = util::read_file(&path_4xx) {
            static_file.body_4xx = bytes;
        }
    }

    if path_5xx.exists() {
        if let Ok(bytes) = util::read_file(&path_5xx) {
            static_file.body_5xx = bytes;
        }
    }

    app.at("/*").get(static_file);
    app.serve(&*DEFAULT_ADDRESS).expect("serving");
}
