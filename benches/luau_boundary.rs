//! The Luau boundary: what a script pays to call into the configuration from Luau.
//!
//! Every scenario is a Luau function that runs its operation `OPS` times per call, so the cost
//! of entering Luau from Rust is amortised away and the reported time divided by `OPS`
//! (Criterion's throughput line) is the per-operation cost. The scripts are the contract: they
//! were frozen at the pre-migration commit (85ffeb3, mlua) and run unchanged against the l3i
//! bindings, except `contentFilesMaterialise`, which now spells the copy out with `:toTable()`
//! because the list methods return views (a documented break).
//!
//! Run with `cargo bench --features luau --bench luau_boundary`.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use l3i::Runtime;
use l3i::call::CallResults;
use l3i::extension::{RuntimePlan, RuntimePolicy};
use l3i::value::{Function, Table, Value};
use openmw_config::luau;
use std::fmt::Write as _;
use std::path::PathBuf;

/// Operations per Luau call.
const OPS: u64 = 1000;

/// The scenarios, returned as a table of functions by one chunk. `rootPath`, `foundKey`, and
/// `hitFile` are globals the harness sets before loading the chunk.
const SCENARIOS: &str = r#"
    local cfg = openmwConfig.new(rootPath)
    local key, file, ops = foundKey, hitFile, ops
    return {
        getGameSettingFound = function()
            local row
            for _ = 1, ops do row = cfg:getGameSetting(key) end
            return row.value
        end,
        getGameSettingMissing = function()
            local row
            for _ = 1, ops do row = cfg:getGameSetting("iMissing") end
            return row
        end,
        hasContentFileHit = function()
            local hit = false
            for _ = 1, ops do hit = cfg:hasContentFile(file) end
            return hit
        end,
        hasContentFileMiss = function()
            local hit = true
            for _ = 1, ops do hit = cfg:hasContentFile("NonExistent.esp") end
            return hit
        end,
        contentFilesMaterialise = function()
            local n = 0
            for _ = 1, ops do n = #cfg:contentFiles():toTable() end
            return n
        end,
        contentFilesIndex = function()
            local n = 0
            for _ = 1, ops do
                local list = cfg:contentFiles()
                for i = 1, #list do
                    if list[i] then n = n + 1 end
                end
            end
            return n
        end,
        contentFilesFor = function()
            local n = 0
            for _ = 1, ops do
                for _, name in cfg:contentFiles() do
                    if name then n = n + 1 end
                end
            end
            return n
        end,
        gameSettingsRows = function()
            local n = 0
            for _ = 1, ops do
                for _, row in cfg:gameSettings() do
                    if row.kind == "Int" then n = n + 1 end
                end
            end
            return n
        end,
        fromEnv = function()
            return openmwConfig.fromEnv():hasContentFile(file)
        end,
    }
"#;

fn write_cfg(dir: &std::path::Path, n_content: usize, n_fallback: usize) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let mut s = String::new();
    for i in 0..n_content {
        let _ = writeln!(s, "content=Plugin{i:04}.esp");
    }
    for i in 0..n_fallback {
        let _ = writeln!(s, "fallback=iSetting{i},{i}");
    }
    let path = dir.join("openmw.cfg");
    std::fs::write(&path, s).unwrap();
    path
}

fn temp_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("omw_luau_boundary_{}_{tag}", std::process::id()))
}

/// A VM from a plan with the module exposed as the `openmwConfig` compatibility global (the
/// frozen scripts predate `require`), and the scenario table loaded for a config of
/// `n_content` plugins and `n_fallback` game settings.
fn scenarios(n_content: usize, n_fallback: usize) -> (Runtime, Table) {
    let dir = temp_dir(&format!("{n_content}_{n_fallback}"));
    let cfg = write_cfg(&dir, n_content, n_fallback);
    // `fromEnv` reads the root config from the environment; the harness is single-threaded.
    // SAFETY: no other thread reads or writes the environment while the benchmark runs.
    unsafe { std::env::set_var("OPENMW_CONFIG", &cfg) };
    let policy = RuntimePolicy::new().compat_global(luau::MODULE, "openmwConfig");
    let plan = RuntimePlan::builder()
        .policy(policy)
        .extension(luau::extension())
        .finalize()
        .unwrap();
    let runtime = Runtime::from_plan(&plan).unwrap();
    runtime
        .set_global("rootPath", &dir.display().to_string())
        .unwrap();
    runtime
        .set_global(
            "foundKey",
            &format!("iSetting{}", n_fallback.saturating_sub(1)),
        )
        .unwrap();
    runtime
        .set_global(
            "hitFile",
            &format!("Plugin{:04}.esp", n_content.saturating_sub(1)),
        )
        .unwrap();
    // Exactly representable: a plain number, as the mlua harness set it.
    #[allow(clippy::cast_precision_loss)]
    runtime.set_global("ops", &(OPS as f64)).unwrap();
    // The frozen chunk returns the scenario table; `eval` runs it and reads that result.
    let table: Table = runtime.eval(SCENARIOS).unwrap();
    (runtime, table)
}

/// The scenario function `name`, ready to call on `runtime`.
struct Scenario<'r> {
    runtime: &'r Runtime,
    function: Function,
}

impl Scenario<'_> {
    fn call<R: CallResults>(&self) -> R {
        self.function.invoke(&self.runtime.stack(), ()).unwrap()
    }
}

fn scenario<'r>(runtime: &'r Runtime, table: &Table, name: &str) -> Scenario<'r> {
    let value: Value = table.get(&runtime.stack(), name).unwrap();
    Scenario {
        runtime,
        function: Function::from_value(value).unwrap(),
    }
}

fn bench_lookups(c: &mut Criterion) {
    let mut group = c.benchmark_group("luau/getGameSetting");
    group.throughput(Throughput::Elements(OPS));
    for n in [50usize, 500, 2000] {
        let (runtime, table) = scenarios(0, n);
        let found = scenario(&runtime, &table, "getGameSettingFound");
        group.bench_with_input(BenchmarkId::new("found", n), &n, |b, _| {
            b.iter(|| found.call::<String>());
        });
        if n == 2000 {
            let missing = scenario(&runtime, &table, "getGameSettingMissing");
            group.bench_with_input(BenchmarkId::new("missing", n), &n, |b, _| {
                b.iter(|| missing.call::<Option<Value>>());
            });
        }
    }
    group.finish();

    let mut group = c.benchmark_group("luau/hasContentFile");
    group.throughput(Throughput::Elements(OPS));
    let (runtime, table) = scenarios(500, 0);
    let hit = scenario(&runtime, &table, "hasContentFileHit");
    let miss = scenario(&runtime, &table, "hasContentFileMiss");
    group.bench_function("hit/500", |b| {
        b.iter(|| hit.call::<bool>());
    });
    group.bench_function("miss/500", |b| {
        b.iter(|| miss.call::<bool>());
    });
    group.finish();
}

fn bench_lists(c: &mut Criterion) {
    let mut group = c.benchmark_group("luau/contentFiles");
    group.throughput(Throughput::Elements(OPS));
    let (runtime, table) = scenarios(500, 500);
    for name in [
        "contentFilesMaterialise",
        "contentFilesIndex",
        "contentFilesFor",
    ] {
        let function = scenario(&runtime, &table, name);
        group.bench_function(format!("{name}/500"), |b| {
            b.iter(|| function.call::<f64>());
        });
    }
    group.finish();

    let mut group = c.benchmark_group("luau/gameSettings");
    group.throughput(Throughput::Elements(OPS));
    let rows = scenario(&runtime, &table, "gameSettingsRows");
    group.bench_function("rows/500", |b| {
        b.iter(|| rows.call::<f64>());
    });
    group.finish();
}

fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("luau/fromEnv");
    let (runtime, table) = scenarios(500, 500);
    let from_env = scenario(&runtime, &table, "fromEnv");
    group.bench_function("load/500", |b| {
        b.iter(|| from_env.call::<bool>());
    });
    group.finish();
}

criterion_group!(benches, bench_lookups, bench_lists, bench_load);
criterion_main!(benches);
