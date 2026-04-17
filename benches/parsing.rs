use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use openmw_config::{DirectorySetting, OpenMWConfiguration};
use std::fmt::Write as FmtWrite;
use std::io::Write;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn write_cfg_to_dir(dir: &std::path::Path, contents: &str) {
    let mut f = std::fs::File::create(dir.join("openmw.cfg")).unwrap();
    f.write_all(contents.as_bytes()).unwrap();
}

fn make_temp_dir(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("omw_bench_{tag}"));
    std::fs::create_dir_all(&base).ok();
    base
}

// ---------------------------------------------------------------------------
// parse_data_directory — the hot inner loop during config load
// ---------------------------------------------------------------------------

fn bench_parse_relative_path(c: &mut Criterion) {
    let config = PathBuf::from("/home/user/.config/openmw");
    let mut comment = String::new();

    c.bench_function("DirectorySetting::new relative", |b| {
        b.iter(|| {
            comment.clear();
            DirectorySetting::new("Data Files", config.clone(), &mut comment)
        });
    });
}

fn bench_parse_absolute_path(c: &mut Criterion) {
    let config = PathBuf::from("/home/user/.config/openmw");
    let mut comment = String::new();

    c.bench_function("DirectorySetting::new absolute", |b| {
        b.iter(|| {
            comment.clear();
            DirectorySetting::new("/absolute/path/to/Data Files", config.clone(), &mut comment)
        });
    });
}

fn bench_parse_quoted_path(c: &mut Criterion) {
    let config = PathBuf::from("/home/user/.config/openmw");
    let mut comment = String::new();

    c.bench_function("DirectorySetting::new quoted", |b| {
        b.iter(|| {
            comment.clear();
            DirectorySetting::new("\"Data Files with spaces\"", config.clone(), &mut comment)
        });
    });
}

fn bench_parse_userdata_token(c: &mut Criterion) {
    let config = PathBuf::from("/home/user/.config/openmw");
    let mut comment = String::new();

    c.bench_function("DirectorySetting::new ?userdata? token", |b| {
        b.iter(|| {
            comment.clear();
            DirectorySetting::new("?userdata?/data", config.clone(), &mut comment)
        });
    });
}

// ---------------------------------------------------------------------------
// Config load — wall-clock cost for representative configs
// ---------------------------------------------------------------------------

fn build_cfg_string(n_data: usize, n_content: usize, n_fallback: usize) -> String {
    let mut s = String::new();
    for i in 0..n_data {
        let _ = writeln!(s, "data=/data/dir{i}");
    }
    for i in 0..n_content {
        let _ = writeln!(s, "content=Plugin{i:04}.esp");
    }
    for i in 0..n_fallback {
        let _ = writeln!(s, "fallback=iSetting{i},{i}");
    }
    s
}

fn bench_load_small(c: &mut Criterion) {
    let dir = make_temp_dir("small");
    write_cfg_to_dir(&dir, &build_cfg_string(10, 10, 50));

    c.bench_function("OpenMWConfiguration::new small (10 dirs, 10 plugins, 50 fallbacks)", |b| {
        b.iter(|| OpenMWConfiguration::new(Some(dir.clone())).unwrap());
    });
}

fn bench_load_medium(c: &mut Criterion) {
    let dir = make_temp_dir("medium");
    write_cfg_to_dir(&dir, &build_cfg_string(50, 100, 500));

    c.bench_function("OpenMWConfiguration::new medium (50 dirs, 100 plugins, 500 fallbacks)", |b| {
        b.iter(|| OpenMWConfiguration::new(Some(dir.clone())).unwrap());
    });
}

fn bench_load_large(c: &mut Criterion) {
    let dir = make_temp_dir("large");
    write_cfg_to_dir(&dir, &build_cfg_string(200, 500, 2000));

    c.bench_function("OpenMWConfiguration::new large (200 dirs, 500 plugins, 2000 fallbacks)", |b| {
        b.iter(|| OpenMWConfiguration::new(Some(dir.clone())).unwrap());
    });
}

// ---------------------------------------------------------------------------
// Query operations
// ---------------------------------------------------------------------------

fn bench_has_content_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("has_content_file");

    for n in [10usize, 100, 500] {
        let dir = make_temp_dir(&format!("has_cf_{n}"));
        write_cfg_to_dir(&dir, &build_cfg_string(0, n, 0));
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        group.bench_with_input(BenchmarkId::new("found", n), &n, |b, _| {
            b.iter(|| config.has_content_file(&format!("Plugin{:04}.esp", n - 1)));
        });

        group.bench_with_input(BenchmarkId::new("not_found", n), &n, |b, _| {
            b.iter(|| config.has_content_file("NonExistent.esp"));
        });
    }
    group.finish();
}

fn bench_get_game_setting(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_game_setting");

    for n in [50usize, 500, 2000] {
        let dir = make_temp_dir(&format!("ggs_{n}"));
        write_cfg_to_dir(&dir, &build_cfg_string(0, 0, n));
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        group.bench_with_input(BenchmarkId::new("found_last", n), &n, |b, _| {
            b.iter(|| config.get_game_setting(&format!("iSetting{}", n - 1)));
        });

        group.bench_with_input(BenchmarkId::new("not_found", n), &n, |b, _| {
            b.iter(|| config.get_game_setting("iMissing"));
        });
    }
    group.finish();
}

fn bench_game_settings_dedup(c: &mut Criterion) {
    let mut group = c.benchmark_group("game_settings dedup");

    // Build a config where each key appears twice (worst-case dedup)
    for n_unique in [50usize, 250, 1000] {
        let dir = make_temp_dir(&format!("dedup_{n_unique}"));
        let mut s = String::new();
        for i in 0..n_unique {
            let _ = writeln!(s, "fallback=iKey{i},1");
            let _ = writeln!(s, "fallback=iKey{i},2");
        }
        write_cfg_to_dir(&dir, &s);
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        group.bench_with_input(BenchmarkId::new("unique_keys", n_unique), &n_unique, |b, _| {
            b.iter(|| config.game_settings().count());
        });
    }
    group.finish();
}

fn bench_content_files_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_files_iter");

    for n in [10usize, 100, 500] {
        let dir = make_temp_dir(&format!("cfi_{n}"));
        write_cfg_to_dir(&dir, &build_cfg_string(0, n, 0));
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        group.bench_with_input(BenchmarkId::new("collect", n), &n, |b, _| {
            b.iter(|| config.content_files_iter().count());
        });
    }
    group.finish();
}

// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_parse_relative_path,
    bench_parse_absolute_path,
    bench_parse_quoted_path,
    bench_parse_userdata_token,
    bench_load_small,
    bench_load_medium,
    bench_load_large,
    bench_has_content_file,
    bench_get_game_setting,
    bench_game_settings_dedup,
    bench_content_files_iter,
);
criterion_main!(benches);
