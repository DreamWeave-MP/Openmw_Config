# Changelog

## Unreleased

- [c118293](https://github.com/DreamWeave-MP/Openmw_Config/commit/c118293) - FIX: Correct race condition between benchmark and changelog jobs
- [7698bf8](https://github.com/DreamWeave-MP/Openmw_Config/commit/7698bf8) - docs: Update CHANGELOG.md [skip ci]
- [1aeff52](https://github.com/DreamWeave-MP/Openmw_Config/commit/1aeff52) - FEAT: Add a benchmark-generating script which outputs mermaid format docs to a BENCHMARKS.md to demonstrate continuous testing
- [7797f96](https://github.com/DreamWeave-MP/Openmw_Config/commit/7797f96) - docs: Update CHANGELOG.md [skip ci]
- [0abf3b3](https://github.com/DreamWeave-MP/Openmw_Config/commit/0abf3b3) - CLEANUP: Update readme
- [94c610e](https://github.com/DreamWeave-MP/Openmw_Config/commit/94c610e) - docs: Update CHANGELOG.md [skip ci]
- [ea3ac87](https://github.com/DreamWeave-MP/Openmw_Config/commit/ea3ac87) - CLEANUP: Add doctests
- [007f360](https://github.com/DreamWeave-MP/Openmw_Config/commit/007f360) - CLEANUP: Update readme
- [d336969](https://github.com/DreamWeave-MP/Openmw_Config/commit/d336969) - FEAT: Add cargo publish --dry-run to the pipeline and auto-generated changelogs
- [3a68223](https://github.com/DreamWeave-MP/Openmw_Config/commit/3a68223) - FEAT: Add a CI pipeline for clippy, tests, and uploading docs to pages
- [353a5e9](https://github.com/DreamWeave-MP/Openmw_Config/commit/353a5e9) - FEAT: Add an MSRV
- [f0a1297](https://github.com/DreamWeave-MP/Openmw_Config/commit/f0a1297) - FIX: Forgot to replace deleted method
- [f94bf56](https://github.com/DreamWeave-MP/Openmw_Config/commit/f94bf56) - CLEANUP: ALL the clippy lints
- [77a964f](https://github.com/DreamWeave-MP/Openmw_Config/commit/77a964f) - FIX: Actually deduplicate game settings by key
- [e17bd81](https://github.com/DreamWeave-MP/Openmw_Config/commit/e17bd81) - CLEANUP: Only do one traversal when getting the user config
- [b3b6b85](https://github.com/DreamWeave-MP/Openmw_Config/commit/b3b6b85) - CLEANUP: Use less to_string
- [1f7580c](https://github.com/DreamWeave-MP/Openmw_Config/commit/1f7580c) - CLEANUP: Clippy lints
- [7ef170c](https://github.com/DreamWeave-MP/Openmw_Config/commit/7ef170c) - BREAK: Remove vec-creating methods; just collect yourself if you need it
- [2ac7468](https://github.com/DreamWeave-MP/Openmw_Config/commit/2ac7468) - CLEANUP: Use COW strings where possible in gamesetting
- [d7760ab](https://github.com/DreamWeave-MP/Openmw_Config/commit/d7760ab) - CLEANUP: More obvious impl
- [1b08978](https://github.com/DreamWeave-MP/Openmw_Config/commit/1b08978) - CLEANUP: Use is_err instead of !is_ok
- [cf5f08c](https://github.com/DreamWeave-MP/Openmw_Config/commit/cf5f08c) - CLEANUP: Emit an actual error if the root config somehow doesn't have a parent directory
- [44dfb9d](https://github.com/DreamWeave-MP/Openmw_Config/commit/44dfb9d) - FIX: Don't allocate a whole new vector just to get the last element of it
- [5880eca](https://github.com/DreamWeave-MP/Openmw_Config/commit/5880eca) - CLEANUP: Make parse_data_directory only require &str
- [4e95478](https://github.com/DreamWeave-MP/Openmw_Config/commit/4e95478) - FIX: Allocate less strings in Display impls
- [edd447f](https://github.com/DreamWeave-MP/Openmw_Config/commit/edd447f) - FIX: Use an O(1) hashset for seen names instead of actually iterating the *entire* config
- [c1d4ac9](https://github.com/DreamWeave-MP/Openmw_Config/commit/c1d4ac9) - FIX: Singleton setter should use rposition
- [971f944](https://github.com/DreamWeave-MP/Openmw_Config/commit/971f944) - FIX: Incorrect source config in certain setter functions
- [3cb53df](https://github.com/DreamWeave-MP/Openmw_Config/commit/3cb53df) - FEAT: Add benchmarks also
- [57a99a0](https://github.com/DreamWeave-MP/Openmw_Config/commit/57a99a0) - FEAT: Add an excessive amount of tests
- [829dca7](https://github.com/DreamWeave-MP/Openmw_Config/commit/829dca7) - FIX: Forgot to correct these tests after reverting the behavior
- [f6ff2a9](https://github.com/DreamWeave-MP/Openmw_Config/commit/f6ff2a9) - FIX: Typo in CARGO_PKG_VERSION

## 0.1.93

- [1994d84](https://github.com/DreamWeave-MP/Openmw_Config/commit/1994d84) - VER: Bump to 0.1.93
- [8d9ee4d](https://github.com/DreamWeave-MP/Openmw_Config/commit/8d9ee4d) - FIX: Follow symlinks when validating config paths
- [b2e721f](https://github.com/DreamWeave-MP/Openmw_Config/commit/b2e721f) - FIX: Absolutize relative config paths and make an attempt to handle invalid/empty ones.
- [cd6906b](https://github.com/DreamWeave-MP/Openmw_Config/commit/cd6906b) - FIX: Also apply more defensive checks when constructing a config from env
- [41e364c](https://github.com/DreamWeave-MP/Openmw_Config/commit/41e364c) - FIX: Be more defensive about empty (and invalid???) paths when constructing a config, and absolutize relative paths upon construction
- [5647e0c](https://github.com/DreamWeave-MP/Openmw_Config/commit/5647e0c) - CLEANUP: Saner handling for serialized path validation
- [32e5f06](https://github.com/DreamWeave-MP/Openmw_Config/commit/32e5f06) - CLEANUP: Make path separators a constant char array
- [86fd4da](https://github.com/DreamWeave-MP/Openmw_Config/commit/86fd4da) - CLEANUP: Get rid of strip_special components because it destroys paths
- [448c8a7](https://github.com/DreamWeave-MP/Openmw_Config/commit/448c8a7) - Merge pull request #2 from benjaminwinger/encoding-value

## 0.1.92

- [79abc96](https://github.com/DreamWeave-MP/Openmw_Config/commit/79abc96) - CLEANUP: Bump to 0.1.92
- [2e836a6](https://github.com/DreamWeave-MP/Openmw_Config/commit/2e836a6) - FIX: vfs-mw is explicitly added by the local/global config, not as a consequence of `resources` being defined

## 0.1.91

- [a5f3234](https://github.com/DreamWeave-MP/Openmw_Config/commit/a5f3234) - bump version

## 0.1.9

- [7faa483](https://github.com/DreamWeave-MP/Openmw_Config/commit/7faa483) - Bump version

## 0.1.8

- [2fa2fd5](https://github.com/DreamWeave-MP/Openmw_Config/commit/2fa2fd5) - Bump version

## 0.1.3

- [63d568b](https://github.com/DreamWeave-MP/Openmw_Config/commit/63d568b) - CLEANUP: Add GPL notices
- [a6bba67](https://github.com/DreamWeave-MP/Openmw_Config/commit/a6bba67) - CLEANUP: Update URL, bump version, explicit GPL
- [0240c09](https://github.com/DreamWeave-MP/Openmw_Config/commit/0240c09) - FIX: Bump version for important reserialization fix
- [ae5afd2](https://github.com/DreamWeave-MP/Openmw_Config/commit/ae5afd2) - FIX: Correct superbad issue where reserializing configs was all on one line lmao
- [78cffb3](https://github.com/DreamWeave-MP/Openmw_Config/commit/78cffb3) - Bump Version
- [69b7408](https://github.com/DreamWeave-MP/Openmw_Config/commit/69b7408) - FIX: Make modules public, and properly add resources/data-local as data directories
- [d5fcf17](https://github.com/DreamWeave-MP/Openmw_Config/commit/d5fcf17) - FIX: Make more components public

## 0.1.2

- [8ae8253](https://github.com/DreamWeave-MP/Openmw_Config/commit/8ae8253) - CLEANUP: Update cargo version
- [d19f273](https://github.com/DreamWeave-MP/Openmw_Config/commit/d19f273) - FIX: Document dir should include my games on windows
- [a9e41e5](https://github.com/DreamWeave-MP/Openmw_Config/commit/a9e41e5) - CLEANUP: Emit errors when parsing directories and don't include the root configuration path when adding subconfig data directories
- [113656e](https://github.com/DreamWeave-MP/Openmw_Config/commit/113656e) - FIX: Remove `openmw.cfg` from the path of any data directories or more importantly, sub-configurations
- [9479bcf](https://github.com/DreamWeave-MP/Openmw_Config/commit/9479bcf) - CLEANUP: Unify and improve handling of possibly-symlinked path entries

## 0.1.1

- [6755897](https://github.com/DreamWeave-MP/Openmw_Config/commit/6755897) - FIX: It's user-data, not userdata
- [d75bfe2](https://github.com/DreamWeave-MP/Openmw_Config/commit/d75bfe2) - CLEANUP: Improve error messages for duplicates
- [7d972d4](https://github.com/DreamWeave-MP/Openmw_Config/commit/7d972d4) - FEAT: Support groundcover= entries natively
- [9789d54](https://github.com/DreamWeave-MP/Openmw_Config/commit/9789d54) - FIX: Correct some erroneous error types
- [b80ce6e](https://github.com/DreamWeave-MP/Openmw_Config/commit/b80ce6e) - CLEANUP: Remove Dead code
- [a2d3d1b](https://github.com/DreamWeave-MP/Openmw_Config/commit/a2d3d1b) - FIX: When adding content files the source config should always refer to the openmw.cfg and not the directory in which it lives
- [9d5f534](https://github.com/DreamWeave-MP/Openmw_Config/commit/9d5f534) - FEAT: Add one last helper function to get the user config after parsing the first one to ensure the most-writable config is always loaded
- [f618173](https://github.com/DreamWeave-MP/Openmw_Config/commit/f618173) - FEAT: Add more helper methods for basic interactions and restore uber-simplified serialization methods
- [ad3467e](https://github.com/DreamWeave-MP/Openmw_Config/commit/ad3467e) - FEAT: Add some more basic helper functions for content files and archives
- [f5b6300](https://github.com/DreamWeave-MP/Openmw_Config/commit/f5b6300) - FIX: Also include newlines in queued comments for empty lines
- [609bc0d](https://github.com/DreamWeave-MP/Openmw_Config/commit/609bc0d) - FEAT: Actually use the setting value ctors for content files and archives
- [eae8fa6](https://github.com/DreamWeave-MP/Openmw_Config/commit/eae8fa6) - FEAT: Add a filesetting type and an error for duplicate archive entries
- [b7141d9](https://github.com/DreamWeave-MP/Openmw_Config/commit/b7141d9) - FEAT: Implement support for generic settings
- [8bb94cb](https://github.com/DreamWeave-MP/Openmw_Config/commit/8bb94cb) - FIX: Strip . and .. components and add appropriate tests
- [354ec31](https://github.com/DreamWeave-MP/Openmw_Config/commit/354ec31) - FIX: get singleton settings by iterating the map in reverse, to get latest-overriding values
- [a0f24db](https://github.com/DreamWeave-MP/Openmw_Config/commit/a0f24db) - FIX: Correct a bug where parsing directories could fail weirdly if the prefix of a token was a root path and join would destroy the path
- [9047128](https://github.com/DreamWeave-MP/Openmw_Config/commit/9047128) - CLEANUP: Add tests for directorysetting, format
- [d2d1c27](https://github.com/DreamWeave-MP/Openmw_Config/commit/d2d1c27) - FIX: Correct newline output in encodingsetting.rs
- [a1c9692](https://github.com/DreamWeave-MP/Openmw_Config/commit/a1c9692) - FEAT: Add tests for gamesettings
- [3dcc7ff](https://github.com/DreamWeave-MP/Openmw_Config/commit/3dcc7ff) - CLEANUP: Un-support trailing comments and improve root config path initialization, add key and value functions to gamesetting, improve gamesetting ergonomics without actually having to destroy or clone them
- [52cc8cf](https://github.com/DreamWeave-MP/Openmw_Config/commit/52cc8cf) - CLEANUP: Remove unnecessary fields, integrate the rest of the directory settings, gut serialization code
- [b048552](https://github.com/DreamWeave-MP/Openmw_Config/commit/b048552) - CLEANUP: Use `take` for pushing subconfig comments
- [adbd72f](https://github.com/DreamWeave-MP/Openmw_Config/commit/adbd72f) - CLEANUP: Dead code
- [7764e4f](https://github.com/DreamWeave-MP/Openmw_Config/commit/7764e4f) - CLEANUP: Format
- [6375114](https://github.com/DreamWeave-MP/Openmw_Config/commit/6375114) - FEAT: Add partialEq against &str values for GameSettingType
- [be1c512](https://github.com/DreamWeave-MP/Openmw_Config/commit/be1c512) - IMPROVEMENT: Make set_game_setting take a string value (as it would have existed in openmw.cfg) and an optional Path, defaulting to the userconfig dir if none exists
- [21ec006](https://github.com/DreamWeave-MP/Openmw_Config/commit/21ec006) - FEAT: Add a setter function for game settings so that duplicates cannot be a thing
- [39d1e45](https://github.com/DreamWeave-MP/Openmw_Config/commit/39d1e45) - FEAT: Add sub-configurations into the global settings map
- [3eefcc1](https://github.com/DreamWeave-MP/Openmw_Config/commit/3eefcc1) - FEAT: Add explicit methods to retrieve the configuration root file *or* directory
- [86160f8](https://github.com/DreamWeave-MP/Openmw_Config/commit/86160f8) - CLEANUP: Minor optimizations for user_config_path - now consumes the iterator, so never use on the actual self.settings
- [8570e83](https://github.com/DreamWeave-MP/Openmw_Config/commit/8570e83) - FIX: Don't throw on symlinks
- [67f5eb4](https://github.com/DreamWeave-MP/Openmw_Config/commit/67f5eb4) - FEAT: Fully migrate over to DirectorySettings
- [196bf98](https://github.com/DreamWeave-MP/Openmw_Config/commit/196bf98) - FEAT: Correctly take comments into account, eufuckulate display methods of gamesettings, throw on invalid pairs instead of treating them as comment strings
- [563303a](https://github.com/DreamWeave-MP/Openmw_Config/commit/563303a) - FEAT: Major refactor, incllude macros for singleton settings and store them in the global settings map
- [2f535d8](https://github.com/DreamWeave-MP/Openmw_Config/commit/2f535d8) - FEAT: Add function to get default data-local dir
- [9c2c38b](https://github.com/DreamWeave-MP/Openmw_Config/commit/9c2c38b) - FEAT: Implement custom error type and use it throughout each existing setting type
- [150cf09](https://github.com/DreamWeave-MP/Openmw_Config/commit/150cf09) - IMPROVEMENT: Provide the key of a gameSetting constructor to it
- [af7f0d9](https://github.com/DreamWeave-MP/Openmw_Config/commit/af7f0d9) - CLEANUP: Migrate fallback= entries over to game_settings entirely
- [fde7fac](https://github.com/DreamWeave-MP/Openmw_Config/commit/fde7fac) - FEAT: Add a directorySetting module and switch userdata onto it
- [aeba5dd](https://github.com/DreamWeave-MP/Openmw_Config/commit/aeba5dd) - FEAT: Log more info with verbose env
- [1de90f7](https://github.com/DreamWeave-MP/Openmw_Config/commit/1de90f7) - FIX: Correct listed priority order of configurations
- [c60e5bc](https://github.com/DreamWeave-MP/Openmw_Config/commit/c60e5bc) - CLEANUP: Formatting
- [1eb6a5f](https://github.com/DreamWeave-MP/Openmw_Config/commit/1eb6a5f) - CLEANUP: More concide userdata impl
- [3029a60](https://github.com/DreamWeave-MP/Openmw_Config/commit/3029a60) - CLEANUP: Use a constant for no config dir
- [ae38fd9](https://github.com/DreamWeave-MP/Openmw_Config/commit/ae38fd9) - CLEANUP: Move more functions out of core config
- [0e2be14](https://github.com/DreamWeave-MP/Openmw_Config/commit/0e2be14) - FEAT: Add a dedicated setting type for fallback= entries
- [768c47f](https://github.com/DreamWeave-MP/Openmw_Config/commit/768c47f) - CLEANUP: Move parse_data_directory into strings and split out a new util module
- [0bf0028](https://github.com/DreamWeave-MP/Openmw_Config/commit/0bf0028) - CLEANUP: Move default config path functions into lib.rs
- [eb4dae3](https://github.com/DreamWeave-MP/Openmw_Config/commit/eb4dae3) - CLEANUP: Make strings module its own file

