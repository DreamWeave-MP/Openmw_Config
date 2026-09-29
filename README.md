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

With the `lua` feature, a Rust host can give scripts the same configuration:

```toml
[dependencies]
openmw-config = { version = "2", features = ["lua"] }
mlua = { version = "0.12", default-features = false, features = ["luau"] }
```

```lua
local cfg = openmwConfig.fromEnvOrUserConfig()
cfg:addContentFile("MyPlugin.esp")
cfg:saveUser()
```

## What it does

- Root discovery through `OPENMW_CONFIG`, `OPENMW_CONFIG_DIR`, the executable's directory and the
  global config; `config=` level by level; every `replace=` form; the `?local?`, `?global?`,
  `?userdata?` and `?userconfig?` tokens, Flatpak included.
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
