use crate::staticfile::StaticFile;
use crate::util;
use crate::watcher::PrivateWatcher;
use tide::App;

use std::path::Path;
use std::{env, thread};

lazy_static! {
    static ref DEFAULT_ADDRESS: String =
        env::var("ADDRESS").unwrap_or(String::from("localhost:8000"));
    pub static ref SERVE_PATH_ROOT: String = env::var("SOURCE").unwrap_or(String::from("./source"));
    pub static ref PRIVATE_SERVE_PATH: String = SERVE_PATH_ROOT.clone() + "/private";
    pub static ref PRIVATE_PATH_ROOT: String =
        env::var("PRIVATE_SOURCE").unwrap_or(String::from("./private"));
    pub static ref CONFIG_FILE: String = env::var("CONFIG").unwrap_or(String::from("config.json"));
}

pub fn start() {
    util::prepare_logger();
    util::create_dir_if_not_exists(&*PRIVATE_PATH_ROOT);
    util::create_dir_if_not_exists(&*SERVE_PATH_ROOT);

    info!(
        "Initializing watcher (private source: {}, private serve: {}, config: {}).",
        &*PRIVATE_PATH_ROOT, &*PRIVATE_SERVE_PATH, &*CONFIG_FILE
    );
    let mut watcher = PrivateWatcher::new(&*CONFIG_FILE, &*PRIVATE_PATH_ROOT, &*PRIVATE_SERVE_PATH);
    watcher.initialize();

    let _ = thread::spawn(move || {
        watcher.start_watching();
    });

    let mut app = App::new(());
    info!(
        "Initializing staticfile handler to point to {}",
        &*SERVE_PATH_ROOT
    );
    let mut static_file = StaticFile::new(&*SERVE_PATH_ROOT);

    let (path_4xx, path_5xx) = (
        Path::new(&*SERVE_PATH_ROOT).join("4xx.html"),
        Path::new(&*SERVE_PATH_ROOT).join("5xx.html"),
    );
    if path_4xx.exists() {
        info!("Using custom 4xx.html");
        if let Ok(bytes) = util::read_file(&path_4xx) {
            static_file.body_4xx = bytes;
        }
    }

    if path_5xx.exists() {
        info!("Using custom 5xx.html");
        if let Ok(bytes) = util::read_file(&path_5xx) {
            static_file.body_5xx = bytes;
        }
    }

    app.at("/").get(static_file.clone());
    app.at("/*").get(static_file);
    app.serve(&*DEFAULT_ADDRESS).expect("serving");
}
