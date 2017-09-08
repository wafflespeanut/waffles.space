use {serde_json, utils};
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode};
use server::{CONFIG_FILE, SERVE_PATH_ROOT};
use std::collections::HashMap;
use std::fs::{self, File};
use std::sync::mpsc::{self, Receiver};
use std::path::{Component, Path};
use std::process::Command;
use std::time::Duration;
use uuid::Uuid;

#[allow(dead_code)]
pub struct PrivateWatcher {
    config: HashMap<String, String>,
    root_path: String,
    event_receiver: Receiver<DebouncedEvent>,
    watcher: RecommendedWatcher,
}

impl PrivateWatcher {
    pub fn new(path: &str) -> PrivateWatcher {
        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher =
            Watcher::new(tx, Duration::from_secs(2))
                    .expect("cannot create watcher");
        utils::create_dir_if_not_exists(path);

        watcher.watch(path, RecursiveMode::NonRecursive)
               .expect("cannot watch path");
        info!("Watching {}...", path);

        PrivateWatcher {
            config: HashMap::new(),
            root_path: path.to_owned(),
            event_receiver: rx,
            watcher: watcher,
        }
    }

    fn load_config(&mut self) {
        self.config = File::open(&*CONFIG_FILE).ok().and_then(|mut fd| {
            serde_json::from_reader(&mut fd).ok()
        }).unwrap_or_default();

        let source = Path::new(&*SERVE_PATH_ROOT);
        self.config.retain(|ref parent, token| {
            source.join(token).join(parent).exists()
        });
    }

    #[inline]
    fn dump_config(&self) {
        File::create(&*CONFIG_FILE).ok().and_then(|mut fd| {
            serde_json::to_writer_pretty(&mut fd, &self.config).ok()
        });
    }

    fn reload_source(&mut self, path: &Path) {
        self.load_config();
        let source = Path::new(&*SERVE_PATH_ROOT);
        let rel_path = path.strip_prefix(&self.root_path).unwrap();
        let mut path_items = rel_path.components();
        if let Some(Component::Normal(s)) = path_items.next() {
            let parent = s.to_string_lossy();
            let id = match self.config.get(&*parent) {
                Some(u) => u.to_owned(),
                None => Uuid::new_v4().hyphenated().to_string(),
            };

            let dir_path = source.join(&id);
            if !dir_path.is_dir() {
                fs::create_dir(&dir_path).map_err(|e| {
                    info!("Cannot create {}: {}", dir_path.display(), e);
                }).ok();
            }

            let new_path = dir_path.join(rel_path);
            info!("Copying detected path {} to {}...",
                  &path.display(), &new_path.display());
            Command::new("cp")
                    .args(&["-r", &path.display().to_string(),
                            &new_path.display().to_string()])
                    .output().ok();
            self.config.insert(parent.into_owned(), id);
        } else {
            info!("Path {} doesn't have appropriate component",
                  rel_path.display());
        }

        self.dump_config();
    }

    pub fn start_watching(&mut self) {
        loop {
            match self.event_receiver.recv() {
                Ok(DebouncedEvent::Create(ref path)) =>
                    self.reload_source(path),
                _ => (),
            }
        }
    }
}
