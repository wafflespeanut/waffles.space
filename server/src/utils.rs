use std::fs;
use std::path::Path;

pub fn create_dir_if_not_exists(path: &str) {
    if !Path::new(path).is_dir() {
        info!("Creating {}...", path);
        fs::create_dir_all(path).expect("cannot create directory");
    }
}
