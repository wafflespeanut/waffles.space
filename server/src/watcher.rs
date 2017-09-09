use {serde_json, utils};
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode};
use server::{CONFIG_FILE, PRIVATE_SERVE_PATH};
use std::collections::HashMap;
use std::fs::{self, File};
use std::sync::mpsc::{self, Receiver};
use std::path::{Component, Path};
use std::process::Command;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

pub struct PrivateWatcher {
    config: HashMap<String, String>,
    root_path: String,
    reflect_path: String,
    event_receiver: Receiver<DebouncedEvent>,
    watcher: RecommendedWatcher,
}

impl PrivateWatcher {
    pub fn new(path: &str, reflect_path: &str) -> PrivateWatcher {
        let (tx, rx) = mpsc::channel();
        utils::create_dir_if_not_exists(path);
        utils::create_dir_if_not_exists(reflect_path);

        PrivateWatcher {
            config: HashMap::new(),
            root_path: path.to_owned(),
            reflect_path: reflect_path.to_owned(),
            event_receiver: rx,
            watcher: Watcher::new(tx, Duration::from_secs(2))
                             .expect("cannot create watcher"),
        }
    }

    fn check_private_paths<F>(root: &str, mut call: F)
        where F: FnMut(String, String)          // (token, dir name)
    {
        let source = Path::new(&*PRIVATE_SERVE_PATH);
        for entry in fs::read_dir(&source).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                let token = entry.file_name().to_str().unwrap().to_owned();
                let dir_path = source.join(&token);
                if Uuid::from_str(&token).is_err() {
                    info!("Removing directory with invalid UUID: {}...", dir_path.display());
                    fs::remove_dir_all(&dir_path).expect("removing invalid dir");
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
            }
        }
    }

    fn initialize(&mut self) {
        info!("Cleaning up private directory...");
        if Path::new(&self.reflect_path).exists() {
            fs::remove_dir_all(&self.reflect_path).expect("initial cleanup");
        }

        fs::create_dir(&self.reflect_path).expect("creating private source");
        let source = Path::new(&self.reflect_path).to_owned();

        for entry in fs::read_dir(&self.root_path).unwrap().filter_map(|e| e.ok()) {
            let name = entry.file_name().to_str().unwrap().to_owned();
            let id = Uuid::new_v4().hyphenated().to_string();
            let dir_path = source.join(&id);
            if !dir_path.exists() {
                fs::create_dir(&dir_path).expect("private dir creation");
            }

            let new_path = dir_path.join(&name);
            info!("Copying {} to {}", entry.path().display(), new_path.display());
            Command::new("cp")
                    .args(&["-r", &entry.path().display().to_string(),
                            &new_path.display().to_string()])
                    .output().expect("cp");
        }

        self.reload_config();
        self.watcher.watch(&self.root_path, RecursiveMode::Recursive)
                    .expect("cannot watch path");
        info!("Watching {}...", self.root_path);
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

        let source = Path::new(&*PRIVATE_SERVE_PATH);
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
        let source = Path::new(&*PRIVATE_SERVE_PATH);
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
        self.initialize();

        loop {
            let event = self.event_receiver.recv();
            self.reload_config();

            match event {
                Ok(DebouncedEvent::Create(ref path)) =>
                    self.reflect_source(path),
                Ok(DebouncedEvent::Remove(ref path)) =>
                    self.reflect_source(path),
                _ => (),
            }
        }
    }
}
