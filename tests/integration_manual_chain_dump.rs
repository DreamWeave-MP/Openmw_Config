use openmw_config::OpenMWConfiguration;
use std::path::{Path, PathBuf};

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

#[test]
#[ignore = "manual test that writes local chain file"]
fn dump_real_config_chain_to_repo_local_file() {
    let root_cfg = openmw_config::default_config_path().join("openmw.cfg");

    let config = OpenMWConfiguration::new(Some(root_cfg.clone())).unwrap_or_else(|err| {
        panic!(
            "failed to load config chain from {}: {err}",
            root_cfg.display()
        )
    });

    let mut chain_paths = Vec::new();
    chain_paths.push(
        absolute_path(config.root_config_file()).expect("failed to absolutize root config path"),
    );
    chain_paths.extend(config.sub_configs().map(|sub| {
        absolute_path(&sub.parsed().join("openmw.cfg"))
            .expect("failed to absolutize sub-config path")
    }));

    let output_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("real_config_chain_paths.txt");

    let body = chain_paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<String>>()
        .join("\n");

    std::fs::write(&output_path, format!("{body}\n"))
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output_path.display()));
}
