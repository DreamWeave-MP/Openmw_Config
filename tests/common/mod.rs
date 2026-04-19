use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub fn temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("openmw_cfg_{prefix}_{id}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write_cfg(dir: &Path, contents: &str) -> PathBuf {
    let cfg = dir.join("openmw.cfg");
    let mut file = std::fs::File::create(&cfg).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    cfg
}
