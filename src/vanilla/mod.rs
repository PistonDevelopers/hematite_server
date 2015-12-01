use std::env;
use std::path::PathBuf;

fn var(key: &str) -> String {
    match env::var(key) {
        Ok(val) => val,
        Err(err) => panic!("couldn't find env var {}; err: {:?}", key, err)
    }
}

#[cfg(windows)]
pub fn root_path() -> PathBuf {
    let appdata = var("appdata");
    let mut buf = PathBuf::from(&appdata);
    buf.push(".minecraft");
    buf
}

#[cfg(target_os = "linux")]
pub fn root_path() -> PathBuf {
    let home = var("HOME");
    let mut buf = PathBuf::from(home);
    buf.push(".minecraft");
    buf
}

#[cfg(target_os = "macos")]
fn root_path() -> PathBuf {
    let home = var("HOME");
    let mut buf = PathBuf::from(home);
    buf.push("Library");
    buf.push("Application Support");
    buf.push("minecraft");
    buf
}
