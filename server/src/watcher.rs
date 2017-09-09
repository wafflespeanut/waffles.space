use {serde_json, utils};
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode};
use server::{CONFIG_FILE, SERVE_PATH_ROOT};
use std::collections::HashMap;
use std::fs::{self, File};
use std::sync::mpsc::{self, Receiver};
use std::path::{Component, Path};
use std::str::FromStr;
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
            Watcher::new(tx, Duration::from_secs(2)).expect("cannot create watcher");
        utils::create_dir_if_not_exists(path);

        watcher.watch(path, RecursiveMode::Recursive).expect("cannot watch path");
        info!("Watching {}...", path);

        PrivateWatcher {
            config: HashMap::new(),
            root_path: path.to_owned(),
            event_receiver: rx,
            watcher: watcher,
        }
    }

    fn check_private_paths<F>(root: &str, mut call: F)
        where F: FnMut(String, String)          // (token, dir name)
    {
        let source = Path::new(&*SERVE_PATH_ROOT);
        for entry in fs::read_dir(&source).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let token = entry.file_name().to_str().unwrap().to_owned();
                let dir_path = source.join(&token);
                if Uuid::from_str(&token).is_err() {
                    info!("Removing directory with invalid UUID: {}...", dir_path.display());
                    fs::remove_dir_all(&dir_path).unwrap();
                    continue
                }

                let entries = fs::read_dir(&dir_path).unwrap()
                                 .filter_map(|e| e.ok())
                                 .collect::<Vec<_>>();
                if entries.is_empty() {
                    info!("Removing empty directory: {}", dir_path.display());
                    fs::remove_dir(&dir_path).unwrap();
                } else {
                    if entries.len() > 1 {
                        error!("{} should have atmost one entry! (found: {})",
                               dir_path.display(), entries.len());
                    } else {
                        let entry = entries.get(0).unwrap();
                        let name = entry.file_name().to_str().unwrap().to_owned();
                        if Path::new(&root).join(&name).exists() {
                            call(token, name);
                        } else {
                            info!("{} not found in source. Removing...", name);
                            fs::remove_dir_all(&dir_path).unwrap();
                        }
                    }
                }
            } else {
                error!("{} is not a directory.", path.display());
            }
        }
    }

    /// Load config and check config, source and serve directory.
    fn reload_config(&mut self) {
        self.config = File::open(&*CONFIG_FILE).ok().and_then(|mut fd| {
            serde_json::from_reader(&mut fd).ok()
        }).unwrap_or_default();

        PrivateWatcher::check_private_paths(&self.root_path.clone(), |token, name| {
            if self.config.values().find(|ref t| ***t == token).is_none() {
                info!("Adding {} to config", token);
                self.config.insert(name, token);
            }
        });

        let source = Path::new(&*SERVE_PATH_ROOT);
        self.config.retain(|ref parent, token| {
            source.join(token).join(parent).exists()
        });

        File::create(&*CONFIG_FILE).ok().and_then(|mut fd| {    // overwrite config
            serde_json::to_writer_pretty(&mut fd, &self.config).ok()
        });
    }

    /// Find the head component (file or dir) of the given path.
    fn find_head(&self, relative_path: &Path) -> String {
        for item in relative_path.components() {
            if let Component::Normal(s) = item {
                return s.to_string_lossy().into_owned()
            }
        }

        panic!("Path {} doesn't have appropriate component",
               relative_path.display());
    }

    /// Reflect source from the given `Path` (which should a sub-path of `SERVE_PATH_ROOT`).
    fn reflect_source(&mut self, path: &Path) {
        let source = Path::new(&*SERVE_PATH_ROOT);
        let rel_path = path.strip_prefix(&self.root_path).unwrap();
        let parent = self.find_head(&rel_path);
        let id = match self.config.get(&parent) {
            Some(u) => u.to_owned(),
            None => Uuid::new_v4().hyphenated().to_string(),    // random
        };

        let new_path = source.join(&id).join(rel_path);
        if path.exists() {
            if path.is_dir() {
                if !new_path.exists() {
                    info!("Creating {}", new_path.display());
                    fs::create_dir_all(&new_path).expect("recursive dir creation");
                }
            } else {
                info!("Copying detected path {} to {}...",
                      &path.display(), &new_path.display());
                let parent = new_path.parent().unwrap();
                if !parent.exists() {
                    fs::create_dir_all(&parent).expect("parent dir creation");
                }

                fs::copy(path, &new_path).expect("copying file");
            }
        } else {
            if new_path.exists() {
                if new_path.is_dir() {
                    info!("Removing {}", new_path.display());
                    fs::remove_dir_all(&new_path).expect("recursive dir deletion");
                } else {
                    info!("Removing {}", new_path.display());
                    fs::remove_file(&new_path).expect("");
                }
            }
        }
    }

    pub fn start_watching(&mut self) {
        self.reload_config();

        loop {
            match self.event_receiver.recv() {
                Ok(DebouncedEvent::Create(ref path)) =>
                    self.reflect_source(path),
                Ok(DebouncedEvent::Remove(ref path)) =>
                    self.reflect_source(path),
                _ => (),
            }

            self.reload_config();
        }
    }
}
