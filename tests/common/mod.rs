use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A fresh, empty directory. The name carries the process id as well as a counter, and whatever
/// an earlier run left under the same name is removed first, so a test that expects a path not
/// to exist yet never meets a previous run's leftovers.
pub fn temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("openmw_cfg_{prefix}_{}_{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

pub fn write_cfg(dir: &Path, contents: &str) -> PathBuf {
    let cfg = dir.join("openmw.cfg");
    let mut file = std::fs::File::create(&cfg).unwrap();
    file.write_all(contents.as_bytes()).unwrap();
    cfg
}
