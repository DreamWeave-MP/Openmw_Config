# openmw-config

Read, compose and write [OpenMW](https://openmw.org/)'s `openmw.cfg` configuration chains from
Rust, or from Lua embedded in a Rust program.

OpenMW does not read one `openmw.cfg`. It reads a chain: a root config, which points with `config=`
at the user's, which may point further, each file adding to or `replace=`-ing what came before.
openmw-config reads that chain the way OpenMW does and hands you the result as one configuration.
Every setting remembers the file it came from and the comments above it, so saving writes each
change back to the file that owns it.

**Documentation, including the full Rust and Lua API reference:
<https://dreamweave-mp.github.io/Openmw_Config/>**

## Quick start

```sh
cargo add openmw-config
```

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    // OpenMW's root config discovery, falling back to the user's openmw.cfg.
    let mut config = OpenMWConfiguration::from_env_or_user_config()?;

    for plugin in config.content_files_iter() {
        println!("{}", plugin.value());
    }

    config.add_content_file("Better Balmora.esp")?;
    config.save_user()
}
```

With the `luau` feature, a Rust host can give Luau scripts the same configuration. The bindings
are an [l3i](https://github.com/DreamWeave-MP/l3i) extension: the host composes
`openmw_config::luau::extension()` into a `RuntimePlan`, makes runtimes from the plan, and
scripts `require("@dream/openmw-config")`. l3i builds Luau itself with clang and lld, so a host
copies the toolchain lines of l3i's `.cargo/config.toml` into its own, as this repository does;
[l3i's site](https://DreamWeave-MP.github.io/l3i/) explains the toolchain and the plan.

```toml
[dependencies]
openmw-config = { version = "3", features = ["luau"] }
l3i = "1"
```

```rust
use l3i::Runtime;
use l3i::extension::RuntimePlan;

fn main() -> l3i::Result<()> {
    let plan = RuntimePlan::builder()
        .extension(openmw_config::luau::extension())
        .finalize()?;
    let runtime = Runtime::from_plan(&plan)?;
    runtime.exec(
        r#"
        local openmwConfig = require("@dream/openmw-config")
        local cfg = openmwConfig.fromEnvOrUserConfig()
        cfg:addContentFile("MyPlugin.esp")
        for i, name in cfg:contentFiles() do print(i, name) end
        cfg:saveUser()
        "#,
    )
}
```

Lists are live views (`#list`, `list[i]`, `for`, `list:toTable()`), rows are userdata with the
same field names as before, and `cfg.isUserConfig` and the counts are fields. The plan renders
typed definitions for the whole module, so a `--!strict` script checks against it. Version 3.0.0
moved the bindings to l3i: a `getGameSetting` or `hasContentFile` call costs about 0.1 to 0.2 µs
from Luau (from 0.2 to 2 µs), list iteration allocates nothing, and loading a large chain is
several times faster. See the [Lua API](https://dreamweave-mp.github.io/Openmw_Config/docs/lua/)
for the surface and the breaks, and
[Embedding Luau](https://dreamweave-mp.github.io/Openmw_Config/docs/lua-hosts/) for the host side.

## What it does

- Root discovery through `OPENMW_CONFIG`, `OPENMW_CONFIG_DIR`, the executable's directory and the
  global config; `config=` depth first, as OpenMW's loader walks it; every `replace=` form; the
  `?local?`, `?global?`, `?userdata?` and `?userconfig?` tokens, Flatpak included.
- Round trips that keep comments, unknown keys and each value's spelling, written through a
  temporary file.
- A flattened export with resolved paths, for importers.
- Parse errors with file and line, and a record of every file the chain loaded or skipped.

## Where to read next

- [Start here](https://dreamweave-mp.github.io/Openmw_Config/docs/start-here/)
- [Loading](https://dreamweave-mp.github.io/Openmw_Config/docs/loading/): which constructor your
  tool wants
- [Rust API](https://dreamweave-mp.github.io/Openmw_Config/docs/api/) and
  [Lua API](https://dreamweave-mp.github.io/Openmw_Config/docs/lua/)
- [Changelog](https://dreamweave-mp.github.io/Openmw_Config/home/changelog/)

openmw-config handles `openmw.cfg` only; `settings.cfg` is not supported yet. It is not affiliated
with the OpenMW project.

## Support

Has openmw-config been useful to you? Consider
[amplifying the signal](https://ko-fi.com/magicaldave) through ko-fi.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this project is dual licensed as `MIT OR Apache-2.0`, without any additional terms or conditions.
