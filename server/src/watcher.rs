use crate::util;
use chrono::{DateTime, Duration as TimeDelta};
use chrono::offset::Utc;
use notify::{DebouncedEvent, RecommendedWatcher, Watcher, RecursiveMode};
use std::collections::HashMap;
use std::fs::{self, File};
use std::sync::mpsc::{self, Receiver};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

// TODO: This is old and huge. Need to refactor and add tests!

const WATCHER_SLEEP_DURATION_MS: u64 = 1000;

/// Indicates the lifetime of this token.
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
struct TokenRotation {
    minutes: Option<u8>,
    hours: Option<u8>,
    days: Option<u8>,
    weeks: Option<u8>,
    /// Whether to remove the token during next rotation (i.e., on expiry)
    remove_on_expiry: bool,
}

impl TokenRotation {
    /// Get the summed up time delta from this object.
    fn get_delta(&self) -> TimeDelta {
        let mut delta = TimeDelta::zero();
        if let Some(m) = self.minutes {
            delta = delta + TimeDelta::minutes(m as i64);
        }

        if let Some(h) = self.hours {
            delta = delta + TimeDelta::hours(h as i64);
        }

        if let Some(d) = self.days {
            delta = delta + TimeDelta::days(d as i64);
        }

        delta
    }
}

/// Represents a private link. By default, expiry is one day.
#[derive(Clone, Copy, Deserialize, Serialize)]
struct PrivateLink {
    id: Uuid,
    expiry: Option<DateTime<Utc>>,
    rotation: TokenRotation,
}

impl Default for PrivateLink {
    fn default() -> Self {
        let mut rotation = TokenRotation::default();
        rotation.days = Some(1);        // default rotation is 1 day.
        PrivateLink {
            id: Uuid::new_v4(),
            expiry: Some(Utc::now() + rotation.get_delta()),
            rotation,
        }
    }
}

impl PrivateLink {
    /// Get the hyphenated UUID of this link.
    fn get_token(&self) -> String {
        self.id.to_hyphenated().to_string()
    }

    /// Check if this link has expired. If it is, then change the UUID,
    /// and apply rotation (i.e., new expiry timestamp).
    fn change_if_expired(&mut self) -> bool {
        if self.expiry.expect("expected expiry") < Utc::now() {
            let rotation = self.rotation;
            *self = PrivateLink::default();
            self.rotation = rotation;
            return true
        }

        false
    }
}

/// Wrapper around notifier to watch a directory for private resources and generate
/// UUID-based links in the actual source directory.
pub struct PrivateWatcher {
    root_path: PathBuf,
    reflect_path: PathBuf,
    config_path: PathBuf,
    config: HashMap<String, PrivateLink>,
    event_receiver: Receiver<DebouncedEvent>,
    watcher: RecommendedWatcher,
}

impl PrivateWatcher {
    /// Initialize this watcher with a root and reflect path.
    pub fn new(config_path: &str, root_path: &str, reflect_path: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        PrivateWatcher {
            root_path: PathBuf::from(root_path),
            reflect_path: PathBuf::from(reflect_path),
            config_path: PathBuf::from(config_path),
            config: HashMap::new(),
            event_receiver: rx,
            watcher: Watcher::new(tx, Duration::from_secs(2)).expect("cannot create watcher"),
        }
    }

    /// Cleanup, create replicas in the serving directory, and start watching.
    pub fn initialize(&mut self) {
        info!("Cleaning up private directory.");
        if self.reflect_path.exists() {
            util::remove_any_path(&self.reflect_path);
        }

        fs::create_dir(&self.reflect_path).expect("creating source path for private entries");
        self.load_config();

        for entry in fs::read_dir(&self.root_path).unwrap().filter_map(|e| e.ok()) {
            let name = String::from(entry.file_name().to_str().unwrap());
            let link = self.config.entry(name.clone()).or_insert(PrivateLink::default());
            let id = link.get_token();
            let dir_path = self.reflect_path.join(&id);
            util::create_dir_if_not_exists(&dir_path);

            let new_path = dir_path.join(&name);
            info!("Copying {} to {}", entry.path().display(), new_path.display());
            info!("Expiry time set to: {}", link.expiry.unwrap());
            Command::new("cp")
                    .args(&["-r", &entry.path().display().to_string(),
                            &new_path.display().to_string()])
                    .output().expect("recursive copy");
        }

        self.check_config();
        self.watcher.watch(&self.root_path, RecursiveMode::Recursive).expect("cannot watch path");
        info!("Watching {}.", self.root_path.display());
    }

    /// Validate the serving path against its source.
    fn check_private_paths<F>(root: &Path, source: &Path, mut check_expiry: F)
        where F: FnMut(Uuid, String) -> Option<String>
    {
        for entry in fs::read_dir(&source).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                info!("Ignoring {} because it's not a directory.", path.display());
                continue
            }

            let token = String::from(entry.file_name().to_str().unwrap());
            let dir_path = source.join(&token);
            let uuid = match token.parse::<Uuid>() {
                Ok(id) => id,
                Err(_) => {
                    info!("Removing directory with invalid UUID: {}.", dir_path.display());
                    util::remove_any_path(dir_path);
                    continue
                }
            };

            let mut entries = fs::read_dir(&dir_path).expect("dir walking").filter_map(|e| e.ok());
            let first_entry = match (&mut entries).next() {
                Some(e) => e,
                None => {
                    info!("Removing empty directory: {}", dir_path.display());
                    util::remove_any_path(dir_path);
                    continue
                },
            };

            if (&mut entries).next().is_some() {
                error!("{} has more than one entry!", dir_path.display());
                continue
            }

            let name = String::from(first_entry.file_name().to_str().unwrap());
            let actual_path = root.join(&name);
            if actual_path.exists() {
                let new_link = check_expiry(uuid, name.clone());
                if let Some(new_id) = new_link {
                    info!("Generated new token for {}: {}", name, new_id);
                    let new_path = source.join(&new_id);
                    fs::rename(dir_path, new_path).expect("renaming expired dir");
                }
            } else {
                info!("Removing {} from private serving source.", name);
                util::remove_any_path(&dir_path);
            }
        }
    }

    // Load the config from default path - if there's none, or if it has errors,
    // then default to empty.
    fn load_config(&mut self) {
        self.config = File::open(&self.config_path).ok().and_then(|mut fd| {
            serde_json::from_reader(&mut fd).ok()
        }).unwrap_or_else(|| HashMap::new());
    }

    /// Load/reload config and ensure cleanliness in serve directory and config.
    fn check_config(&mut self) {
        // Check the entries in private serve path against the root path,
        // set default expiry, change links (if they've expired), and
        // remove unnecessary entries.
        Self::check_private_paths(&self.root_path.clone(), &self.reflect_path.clone(), |uuid, name| {
            if self.config.get(&name).is_none() {
                // This happens when the config is not a valid JSON, and we've defaulted to empty.
                // At this point, we have no choice but to land on the default rotation for that link.
                let mut link = PrivateLink::default();
                link.id = uuid;
                info!("Adding missing link for {}:{} to config.", name, link.id);
                self.config.insert(name.clone(), link);
            }

            let link = self.config.get_mut(&name).unwrap();
            let has_expired = link.change_if_expired();
            if has_expired {
                info!("Token has expired for {}", name);
                // Check if this link has to be removed on expiry. We're removing the source
                // entirely, which will notify us again, and we'll remove the corresponding
                // entry in config.
                if link.rotation.remove_on_expiry {
                    info!("Removing {} from source path.", name);
                    let path = Path::new(&self.root_path).join(&name);
                    util::remove_any_path(path);
                    return None
                }

                return Some(link.get_token());
            }

            None
        });

        // Check the config for unnecessary entries and remove them.
        let source = &self.reflect_path;
        self.config.retain(|ref parent, link| {
            source.join(link.get_token()).join(parent).exists()
        });

        // dump/overwrite the config
        File::create(&self.config_path).ok().and_then(|mut fd| {
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
        let rel_path = path.strip_prefix(&self.root_path).unwrap();
        let parent = self.find_head(&rel_path);
        let link = self.config.entry(parent).or_insert(PrivateLink::default());
        let id = link.get_token();

        let new_path = self.reflect_path.join(&id).join(rel_path);
        if path.exists() {
            if path.is_dir() {
                if !new_path.exists() {
                    info!("Creating {}", new_path.display());
                    fs::create_dir_all(&new_path).expect("recursive dir creation");
                }
            } else {
                info!("Copying detected path {} to {}",
                      &path.display(), &new_path.display());
                let parent = new_path.parent().unwrap();
                if !parent.exists() {
                    fs::create_dir_all(&parent).expect("parent dir creation");
                }

                fs::copy(path, &new_path).expect("copying file");
            }
        } else {
            if new_path.exists() {
                info!("Removing {}", new_path.display());
                util::remove_any_path(&new_path);
            }
        }
    }

    /// Start watching for events and handle them accordingly.
    pub fn start_watching(mut self) {
        let sleep_duration = Duration::from_millis(WATCHER_SLEEP_DURATION_MS);

        // FIXME: Once `notify` has futures-mpsc support, let's switch to
        // `tokio_core::reactor::Interval` for periodic notifications
        // and select over both the streams (instead of try_recv).
        loop {
            // We're loading the config before handling the events, because
            // `reflect_source` will mutate the config.
            self.load_config();

            match self.event_receiver.try_recv() {
                Ok(DebouncedEvent::Create(ref path))
                | Ok(DebouncedEvent::Remove(ref path))
                | Ok(DebouncedEvent::Write(ref path)) =>
                    self.reflect_source(path),
                Ok(DebouncedEvent::Rename(ref old_path, ref new_path)) => {
                    self.reflect_source(old_path);
                    self.reflect_source(new_path);
                },
                _ => (),
            }

            self.check_config();
            thread::sleep(sleep_duration);
        }
    }
}
