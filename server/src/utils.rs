use std::fs;
use std::path::Path;

pub fn create_dir_if_not_exists<P>(path: P)
    where P: AsRef<Path>
{
    if !path.as_ref().is_dir() {
        info!("Creating {}...", path.as_ref().display());
        fs::create_dir_all(path).expect("cannot create directory");
    }
}

pub fn remove_any_path<P>(path: P)
    where P: AsRef<Path>
{
    if path.as_ref().is_dir() {
        fs::remove_dir_all(path).expect("cannot remove directory");
    } else {
        fs::remove_file(path).expect("cannot remove file");
    }
}
