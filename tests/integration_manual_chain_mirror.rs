use openmw_config::{ConfigChainStatus, OpenMWConfiguration};
use std::path::{Component, Path, PathBuf};

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn sanitize_prefix(prefix: &str) -> String {
    prefix
        .chars()
        .map(|ch| match ch {
            ':' | '\\' | '/' => '_',
            _ => ch,
        })
        .collect()
}

fn absolute_suffix(path: &Path) -> PathBuf {
    let mut suffix = PathBuf::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                suffix.push(format!(
                    "prefix_{}",
                    sanitize_prefix(&prefix.as_os_str().to_string_lossy())
                ));
            }
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => suffix.push("__parent__"),
            Component::Normal(part) => suffix.push(part),
        }
    }

    if suffix.as_os_str().is_empty() {
        suffix.push("_root");
    }

    suffix
}

fn snapshot_relative_path(source_abs: &Path, home: Option<&Path>) -> PathBuf {
    if let Some(home) = home
        && let Ok(home_rel) = source_abs.strip_prefix(home)
    {
        return PathBuf::from("home-relative").join(home_rel);
    }

    PathBuf::from("absolute").join(absolute_suffix(source_abs))
}

#[test]
#[ignore = "manual test that mirrors real chain into repo-local snapshot"]
fn mirror_real_config_chain_to_repo_snapshot() {
    let root_cfg = openmw_config::default_config_path().join("openmw.cfg");
    let config = OpenMWConfiguration::new(Some(root_cfg.clone())).unwrap_or_else(|err| {
        panic!(
            "failed to load config chain from {}: {err}",
            root_cfg.display()
        )
    });

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let snapshot_root = repo_root.join("manual_chain_snapshot");

    if snapshot_root.exists() {
        std::fs::remove_dir_all(&snapshot_root).unwrap_or_else(|err| {
            panic!(
                "failed to clear previous snapshot {}: {err}",
                snapshot_root.display()
            )
        });
    }

    std::fs::create_dir_all(&snapshot_root).unwrap_or_else(|err| {
        panic!(
            "failed to create snapshot root {}: {err}",
            snapshot_root.display()
        )
    });

    let home = std::env::var_os("HOME").map(PathBuf::from);

    let mut loaded_count = 0_usize;
    let mut missing_count = 0_usize;
    let mut manifest = Vec::new();

    for entry in config.config_chain() {
        let source_abs = absolute_path(entry.path())
            .unwrap_or_else(|err| panic!("failed to absolutize {}: {err}", entry.path().display()));

        let snapshot_rel = snapshot_relative_path(&source_abs, home.as_deref());

        match entry.status() {
            ConfigChainStatus::Loaded => {
                loaded_count += 1;
                let destination = snapshot_root.join(&snapshot_rel);
                let parent = destination.parent().unwrap_or_else(|| {
                    panic!(
                        "destination unexpectedly had no parent: {}",
                        destination.display()
                    )
                });

                std::fs::create_dir_all(parent).unwrap_or_else(|err| {
                    panic!(
                        "failed to create destination parent {}: {err}",
                        parent.display()
                    )
                });

                std::fs::copy(&source_abs, &destination).unwrap_or_else(|err| {
                    panic!(
                        "failed to copy {} -> {}: {err}",
                        source_abs.display(),
                        destination.display()
                    )
                });

                manifest.push(format!(
                    "loaded | {} | {}",
                    source_abs.display(),
                    snapshot_rel.display()
                ));
            }
            ConfigChainStatus::SkippedMissing => {
                missing_count += 1;
                manifest.push(format!(
                    "missing | {} | {}",
                    source_abs.display(),
                    snapshot_rel.display()
                ));
            }
        }
    }

    let manifest_path = snapshot_root.join("chain_manifest.txt");
    std::fs::write(&manifest_path, format!("{}\n", manifest.join("\n"))).unwrap_or_else(|err| {
        panic!(
            "failed to write manifest {}: {err}",
            manifest_path.display()
        )
    });

    let summary_path = snapshot_root.join("README.txt");
    let summary = format!(
        "Manual config chain snapshot\n\nsource root: {}\nloaded: {}\nmissing: {}\nmanifest: {}\n",
        root_cfg.display(),
        loaded_count,
        missing_count,
        manifest_path.display()
    );
    std::fs::write(&summary_path, summary)
        .unwrap_or_else(|err| panic!("failed to write summary {}: {err}", summary_path.display()));
}
