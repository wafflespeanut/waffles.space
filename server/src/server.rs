use crate::staticfile::{Responder, StaticFile};
use crate::util;
use crate::watcher::PrivateWatcher;
use tide::{Middleware, Next, Request, Response, Server};
use uuid::Uuid;

use crossbeam_channel::Sender;
use std::path::Path;
use std::{env, thread};

const PRIVATE_PATH_PREFIX: &str = "/private";

lazy_static! {
    static ref DEFAULT_ADDRESS: String =
        env::var("ADDRESS").unwrap_or(String::from("localhost:8000"));
    pub static ref SERVE_PATH_ROOT: String = env::var("SOURCE").unwrap_or(String::from("./source"));
    pub static ref PRIVATE_SERVE_PATH: String = SERVE_PATH_ROOT.clone() + PRIVATE_PATH_PREFIX;
    pub static ref PRIVATE_PATH_ROOT: String =
        env::var("PRIVATE_SOURCE").unwrap_or(String::from("./private"));
    pub static ref CONFIG_FILE: String = env::var("CONFIG").unwrap_or(String::from("config.json"));
}

struct PrivateMiddleware {
    sender: Sender<(Uuid, String)>,
}

#[async_trait::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for PrivateMiddleware {
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        let path = req.url().path();
        if !path.starts_with(PRIVATE_PATH_PREFIX) {
            return Ok(next.run(req).await);
        }

        let mut path_iter = path.split("/");
        let _root = path_iter.next();
        let _prefix = path_iter.next();

        if let (Some(uuid), Some(sub_path)) = (
            path_iter.next().and_then(|v| v.parse::<Uuid>().ok()),
            path_iter.next(),
        ) {
            let _ = self.sender.send((uuid, sub_path.into()));
        }

        Ok(next.run(req).await)
    }
}

async fn fetch_file(req: Request<StaticFile>) -> Result<Response, tide::Error> {
    let responder = Responder::from(&req);
    Ok(responder.stream().await)
}

pub async fn start() {
    util::prepare_logger();
    util::create_dir_if_not_exists(&*PRIVATE_PATH_ROOT);
    util::create_dir_if_not_exists(&*SERVE_PATH_ROOT);

    info!(
        "Initializing watcher (private source: {}, private serve: {}, config: {}).",
        &*PRIVATE_PATH_ROOT, &*PRIVATE_SERVE_PATH, &*CONFIG_FILE
    );
    let mut watcher = PrivateWatcher::new(&*CONFIG_FILE, &*PRIVATE_PATH_ROOT, &*PRIVATE_SERVE_PATH);
    let sender = watcher.initialize();

    let _ = thread::spawn(move || {
        watcher.start_watching();
    });

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

    let mut app = Server::with_state(static_file);
    app.with(PrivateMiddleware { sender });
    app.at("/").get(fetch_file);
    app.at("/*").get(fetch_file);
    app.listen(&*DEFAULT_ADDRESS).await.expect("serving");
}
