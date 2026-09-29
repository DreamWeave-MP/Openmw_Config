//! The Luau surface through l3i: the plan finalizes and its declared types check, scripts
//! reach the module through `require("@dream/openmw-config")`, and the read, mutation,
//! persistence, and error contracts of the old `openmwConfig` global hold on the new views.

#![cfg(feature = "luau")]

use l3i::Runtime;
use l3i::extension::{RuntimePlan, RuntimePolicy};
use openmw_config::luau::{self, MODULE};
use openmw_config::try_default_config_path;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn write_cfg(dir: &Path, contents: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("openmw.cfg"), contents).unwrap();
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("openmw_cfg_luau_{name}_{}", std::process::id()));
    std::fs::create_dir_all(&base).unwrap();
    base
}

unsafe fn clear_config_env() {
    // SAFETY: callers hold ENV_LOCK, so process-global environment mutation is serialized.
    unsafe {
        std::env::remove_var("OPENMW_CONFIG");
        std::env::remove_var("OPENMW_CONFIG_DIR");
        std::env::remove_var("OPENMW_GLOBAL_CONFIG_PATH");
        std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
        std::env::remove_var("OPENMW_FLATPAK_ID");
        std::env::remove_var("FLATPAK_ID");
    }
}

fn restore_env_var(key: &str, value: Option<std::ffi::OsString>) {
    // SAFETY: callers hold ENV_LOCK, so process-global environment mutation is serialized.
    unsafe {
        if let Some(value) = value {
            std::env::set_var(key, value);
        } else {
            std::env::remove_var(key);
        }
    }
}

fn plan() -> Rc<RuntimePlan> {
    RuntimePlan::builder()
        .extension(luau::extension())
        .finalize()
        .unwrap()
}

/// A runtime with the module reachable by `require`, and `globals` set for the script.
fn runtime(globals: &[(&str, String)]) -> Runtime {
    let runtime = Runtime::from_plan(&plan()).unwrap();
    for (name, value) in globals {
        runtime.set_global(name, value).unwrap();
    }
    runtime
}

const PRELUDE: &str = "local openmwConfig = require('@dream/openmw-config')\n";

fn exec(runtime: &Runtime, script: &str) {
    if let Err(error) = runtime.exec(&format!("{PRELUDE}{script}")) {
        panic!("{error}");
    }
}

const READ_SURFACE_SCRIPT: &str = r##"
    local cfg = openmwConfig.new(rootPath)

    assert(type(cfg:rootConfigFile()) == "string")
    assert(type(cfg:rootConfigDir()) == "string")
    assert(type(cfg.isUserConfig) == "boolean")
    assert(cfg.isUserConfig == false, "the root chains to a sub config")
    assert(type(cfg:userConfigPath()) == "string")
    assert(type(cfg:userConfig():toString()) == "string")
    assert(cfg:userConfig().isUserConfig == true)
    assert(tostring(cfg):find("dream.openmw.Config", 1, true))

    assert(cfg:hasContentFile("Root.esm"))
    assert(cfg:hasContentFile("Sub.esm"))
    assert(cfg:hasGroundcoverFile("RootGrass.esp"))
    assert(cfg:hasArchiveFile("Root.bsa"))
    assert(cfg:hasDataDir(expectedDataDir))

    local subConfigs = cfg:subConfigs()
    assert(#subConfigs == 1)
    assert(cfg.subConfigCount == 1)
    assert(subConfigs[1] == subPath, subConfigs[1])
    assert(subConfigs[2] == nil)
    assert(#subConfigs:toTable() == 1)

    local chain = cfg:configChain()
    assert(#chain == 3)
    assert(chain[1].status == "loaded")
    assert(chain[2].status == "skippedMissing")
    assert(chain[3].status == "loaded")
    assert(type(chain[1].depth) == "number")
    assert(chain[1].depth == 0 and chain[2].depth == 1)
    assert(type(chain[1].path) == "string")
    assert(tostring(chain[2]):find("skippedMissing", 1, true))
    local seen = 0
    for i, entry in chain do assert(entry.path == chain[i].path) seen += 1 end
    assert(seen == 3)

    local content = cfg:contentFiles()
    assert(#content == 2 and cfg.contentFileCount == 2)
    assert(content[1] == "Root.esm" and content[2] == "Sub.esm" and content[3] == nil)
    assert(content[0] == nil and content[1.5] == nil)
    local names = {}
    for i, name in content do names[i] = name end
    assert(#names == 2 and names[2] == "Sub.esm")
    local plain = content:toTable()
    assert(#plain == 2 and plain[1] == "Root.esm")
    for _, name in ipairs(plain) do assert(type(name) == "string") end
    local ground = cfg:groundcoverFiles()
    assert(#ground == 1 and cfg.groundcoverFileCount == 1)
    local archives = cfg:fallbackArchives()
    assert(#archives == 1 and cfg.archiveFileCount == 1)
    local dirs = cfg:dataDirectories()
    assert(#dirs >= 1 and cfg.dataDirectoryCount == #dirs)

    assert(string.find(cfg:userData(), expectedUserData) ~= nil)
    assert(string.find(cfg:resources(), expectedResources) ~= nil)
    assert(string.find(cfg:dataLocal(), expectedDataLocal) ~= nil)
    assert(cfg:encoding() == "win1252")

    local settings = cfg:gameSettings()
    assert(#settings == 3 and cfg.gameSettingCount == 3)
    assert(type(settings[1].key) == "string")
    assert(type(settings[1].value) == "string")
    assert(type(settings[1].kind) == "string")
    assert(type(settings[1].source) == "string")
    assert(type(settings[1].comment) == "string")

    local fScale = nil
    for _, row in settings do
        if row.key == "fScale" then
            fScale = row
            break
        end
    end
    assert(fScale ~= nil)
    assert(fScale.source == expectedRootCfg)
    assert(fScale.comment == "# game comment\n")
    assert(fScale.kind == "Float" and fScale.typed == 1.5)
    local rows = settings:toTable()
    assert(#rows == 3 and rows[1].key == "sName", "last-defined first")

    local game = cfg:getGameSetting("iDifficulty")
    assert(game ~= nil)
    assert(game.key == "iDifficulty")
    assert(game.value == "20")
    assert(game.kind == "Int")
    assert(game.typed == 20i, "an Int reads as a Luau integer")
    assert(type(game.source) == "string")
    assert(type(game.comment) == "string")
    assert(tostring(game) == "dream.openmw.GameSetting(iDifficulty=20)")
    assert(cfg:getGameSetting("does.not.exist") == nil)
    assert(cfg:getGameSetting("sName").typed == "Hello")

    local generic = cfg:genericSettings()
    assert(#generic == 1 and cfg.genericSettingCount == 1)
    assert(generic[1].key == "no-sound")
    assert(generic[1].value == "1")
    assert(generic[1].source == expectedRootCfg)
    assert(generic[1].comment == "# generic comment\n")
    assert(tostring(generic[1]) == "dream.openmw.GenericSetting(no-sound=1)")

    assert(type(cfg:toString()) == "string")
    assert(type(cfg:toResolvedString()) == "string")
"##;

#[test]
fn module_exports_and_default_helpers() {
    let runtime = runtime(&[]);
    exec(
        &runtime,
        r#"
        assert(type(openmwConfig.version) == "string")

        assert(type(openmwConfig.defaultConfigPath()) == "string")
        assert(type(openmwConfig.defaultUserConfigFile()) == "string")
        assert(type(openmwConfig.defaultUserDataPath()) == "string")
        assert(type(openmwConfig.defaultDataLocalPath()) == "string")
        assert(type(openmwConfig.defaultLocalPath()) == "string")
        assert(type(openmwConfig.fromEnvOrUserConfig) == "function")

        local globalPath, globalErr = openmwConfig.tryDefaultGlobalPath()
        assert((globalPath ~= nil and globalErr == nil) or (globalPath == nil and globalErr ~= nil))

        local cfgPath, cfgErr = openmwConfig.tryDefaultConfigPath()
        assert((cfgPath ~= nil and cfgErr == nil) or (cfgPath == nil and cfgErr ~= nil))

        local userCfg, userCfgErr = openmwConfig.tryDefaultUserConfigFile()
        assert((userCfg ~= nil and userCfgErr == nil) or (userCfg == nil and userCfgErr ~= nil))

        local rootOrUser, rootOrUserErr = openmwConfig.tryDefaultRootOrUserConfigPath()
        assert((rootOrUser ~= nil and rootOrUserErr == nil) or (rootOrUser == nil and rootOrUserErr ~= nil))

        local dataPath, dataErr = openmwConfig.tryDefaultUserDataPath()
        assert((dataPath ~= nil and dataErr == nil) or (dataPath == nil and dataErr ~= nil))

        local localPath, localErr = openmwConfig.tryDefaultLocalPath()
        assert((localPath ~= nil and localErr == nil) or (localPath == nil and localErr ~= nil))

        assert(type(openmwConfig.newEmpty) == "function")
        assert(type(openmwConfig.loadOptional) == "function")

        -- The module is frozen.
        assert(not pcall(function() openmwConfig.new = nil end))
    "#,
    );
}

#[test]
fn the_host_may_expose_a_compatibility_global() {
    let policy = RuntimePolicy::new().compat_global(MODULE, "openmwConfig");
    let plan = RuntimePlan::builder()
        .policy(policy)
        .extension(luau::extension())
        .finalize()
        .unwrap();
    let runtime = Runtime::from_plan(&plan).unwrap();
    runtime
        .exec("assert(openmwConfig.version == require('@dream/openmw-config').version)")
        .unwrap();
}

#[test]
fn from_env_loader() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = temp_dir("from_env_root");
    write_cfg(&root, "content=FromEnv.esm\n");
    let root_cfg = root.join("openmw.cfg");

    // SAFETY: guarded by a global mutex so no concurrent env mutation occurs in tests.
    unsafe {
        clear_config_env();
        std::env::set_var("OPENMW_CONFIG", &root_cfg);
    }

    let runtime = runtime(&[]);
    let result = runtime.exec(&format!(
        "{PRELUDE}local cfg = openmwConfig.fromEnv() assert(cfg:hasContentFile('FromEnv.esm'))"
    ));

    // SAFETY: guarded by a global mutex so no concurrent env mutation occurs in tests.
    unsafe {
        clear_config_env();
    }
    result.unwrap();
}

#[test]
#[cfg(not(windows))]
fn from_env_or_user_config_fallback() {
    let _guard = ENV_LOCK.lock().unwrap();
    let empty_global = temp_dir("luau_empty_global");
    let test_home = temp_dir("luau_home");
    let test_xdg_config_home = temp_dir("luau_xdg_config_home");
    let old_home = std::env::var_os("HOME");
    let old_xdg_config_home = std::env::var_os("XDG_CONFIG_HOME");

    // SAFETY: guarded by a global mutex so no concurrent env mutation occurs in tests.
    unsafe {
        clear_config_env();
        std::env::set_var("OPENMW_GLOBAL_CONFIG_PATH", &empty_global);
        std::env::set_var("HOME", &test_home);
        std::env::set_var("XDG_CONFIG_HOME", &test_xdg_config_home);
    }

    let user_config_dir = try_default_config_path().unwrap();
    write_cfg(&user_config_dir, "content=LuaUserOnly.esm\n");
    let expected_cfg = user_config_dir.join("openmw.cfg");

    let runtime = runtime(&[("expectedCfg", expected_cfg.display().to_string())]);
    let result = runtime.exec(&format!(
        "{PRELUDE}local cfg = openmwConfig.fromEnvOrUserConfig() \
         assert(cfg:rootConfigFile() == expectedCfg) assert(cfg:hasContentFile('LuaUserOnly.esm'))"
    ));

    // SAFETY: guarded by a global mutex so no concurrent env mutation occurs in tests.
    unsafe {
        clear_config_env();
    }
    restore_env_var("HOME", old_home);
    restore_env_var("XDG_CONFIG_HOME", old_xdg_config_home);

    result.unwrap();
}

#[test]
fn read_surface_comprehensive() {
    let root = temp_dir("read_surface_root");
    let sub = temp_dir("read_surface_sub");
    let missing = root.join("does_not_exist_subconfig");
    let userdata_dir = temp_dir("read_surface_userdata");
    let resources_dir = temp_dir("read_surface_resources");
    let data_local_dir = temp_dir("read_surface_data_local");
    let data_dir = temp_dir("read_surface_data");

    write_cfg(
        &root,
        &format!(
            "content=Root.esm\ngroundcover=RootGrass.esp\nfallback-archive=Root.bsa\nencoding=win1252\nuser-data={}\nresources={}\ndata-local={}\ndata={}\nfallback=iDifficulty,20\n# game comment\nfallback=fScale,1.5\nfallback=sName,Hello\n# generic comment\nno-sound=1\nconfig={}\nconfig={}\n",
            userdata_dir.display(),
            resources_dir.display(),
            data_local_dir.display(),
            data_dir.display(),
            sub.display(),
            missing.display()
        ),
    );
    write_cfg(&sub, "content=Sub.esm\n");

    let runtime = runtime(&[
        ("rootPath", root.display().to_string()),
        ("subPath", sub.display().to_string()),
        ("expectedUserData", userdata_dir.display().to_string()),
        ("expectedResources", resources_dir.display().to_string()),
        ("expectedDataLocal", data_local_dir.display().to_string()),
        ("expectedDataDir", data_dir.display().to_string()),
        (
            "expectedRootCfg",
            root.join("openmw.cfg").display().to_string(),
        ),
    ]);
    exec(&runtime, READ_SURFACE_SCRIPT);
}

#[test]
fn typed_values_and_stale_rows() {
    let root = temp_dir("typed_rows_root");
    write_cfg(
        &root,
        "fallback=iColor,10,20,30\nfallback=fWide,2.5\nfallback=iBig,-7\nfallback=sText,a,b\nno-sound=1\n",
    );
    let runtime = runtime(&[("rootPath", root.display().to_string())]);
    exec(
        &runtime,
        r##"
        local cfg = openmwConfig.new(rootPath)
        local color = cfg:getGameSetting("iColor")
        assert(color.kind == "Color" and color.value == "10,20,30")
        assert(color.typed == vector.create(10, 20, 30), tostring(color.typed))
        assert(cfg:getGameSetting("fWide").typed == 2.5)
        assert(cfg:getGameSetting("iBig").typed == -7i)
        assert(cfg:getGameSetting("sText").kind == "String")
        assert(cfg:getGameSetting("sText").typed == "a,b")

        -- Rows read the live configuration and refuse to read a shifted position.
        local row = cfg:getGameSetting("iBig")
        local generic = cfg:genericSettings()[1]
        assert(generic.value == "1")
        cfg:setGameSetting("iNew,1", nil, "added")
        local ok, message = pcall(function() return row.value end)
        assert(not ok and string.find(message, "stale", 1, true), message)
        local okGeneric, messageGeneric = pcall(function() return generic.key end)
        assert(not okGeneric and string.find(messageGeneric, "stale", 1, true), messageGeneric)
        assert(cfg:getGameSetting("iBig").typed == -7i, "a fresh row is fine")
        assert(cfg:getGameSetting("iNew").comment == "# added\n")

        -- Views are live: a mutation shows through an existing view.
        local settings = cfg:gameSettings()
        local before = #settings
        cfg:setGameSetting("iAnother,2", nil, nil)
        assert(#settings == before + 1)
        assert(cfg.gameSettingCount == before + 1)
        local content = cfg:contentFiles()
        assert(#content == 0)
        cfg:addContentFile("Live.esp")
        assert(#content == 1 and content[1] == "Live.esp")
    "##,
    );
}

#[test]
fn mutation_surface_and_persistence() {
    let root = temp_dir("mutate_surface_root");
    let data_dir = temp_dir("mutate_surface_data");
    let user_dir = temp_dir("mutate_surface_userdata");
    let resources_dir = temp_dir("mutate_surface_resources");
    let data_local_dir = temp_dir("mutate_surface_data_local");
    write_cfg(&root, "content=Morrowind.esm\n");

    let runtime = runtime(&[
        ("rootPath", root.display().to_string()),
        ("dataDir", data_dir.display().to_string()),
        ("userDir", user_dir.display().to_string()),
        ("resourcesDir", resources_dir.display().to_string()),
        ("dataLocalDir", data_local_dir.display().to_string()),
    ]);
    exec(
        &runtime,
        r#"
        local cfg = openmwConfig.new(rootPath)

        cfg:addContentFile("LuaMod.esp")
        cfg:addGroundcoverFile("LuaGrass.esp")
        cfg:addArchiveFile("LuaArchive.bsa")
        cfg:addDataDirectory(dataDir)

        assert(cfg:hasContentFile("LuaMod.esp"))
        assert(cfg:hasGroundcoverFile("LuaGrass.esp"))
        assert(cfg:hasArchiveFile("LuaArchive.bsa"))
        assert(cfg:hasDataDir(dataDir))

        cfg:removeContentFile("LuaMod.esp")
        cfg:removeGroundcoverFile("LuaGrass.esp")
        cfg:removeArchiveFile("LuaArchive.bsa")
        cfg:removeDataDirectory(dataDir)

        assert(not cfg:hasContentFile("LuaMod.esp"))
        assert(not cfg:hasGroundcoverFile("LuaGrass.esp"))
        assert(not cfg:hasArchiveFile("LuaArchive.bsa"))
        assert(not cfg:hasDataDir(dataDir))

        cfg:setContentFiles({"A.esm", "B.esp"})
        cfg:setFallbackArchives({"A.bsa"})
        cfg:setDataDirectories({dataDir})
        cfg:setGameSettings({"iDifficulty,10", "fScale,2.0"})
        cfg:setGameSetting("fJumpHeight,1.0", nil, nil)
        assert(cfg:contentFiles()[2] == "B.esp")

        cfg:setUserData(userDir)
        cfg:setResources(resourcesDir)
        cfg:setDataLocal(dataLocalDir)
        cfg:setEncoding("win1251")

        assert(string.find(cfg:userData(), userDir) ~= nil)
        assert(string.find(cfg:resources(), resourcesDir) ~= nil)
        assert(string.find(cfg:dataLocal(), dataLocalDir) ~= nil)
        assert(cfg:encoding() == "win1251")

        cfg:setContentFiles(nil)
        cfg:setFallbackArchives(nil)
        cfg:setDataDirectories(nil)
        cfg:setGameSettings(nil)
        cfg:setUserData(nil)
        cfg:setResources(nil)
        cfg:setDataLocal(nil)
        cfg:setEncoding(nil)

        cfg:setGenericSettings("no-sound", {"2", "3"})
        assert(#cfg:genericSettings() == 2)
        assert(cfg:genericSettings()[1].key == "no-sound")
        assert(cfg:genericSettings()[1].value == "2")

        cfg:addGenericSetting("no-sound", "4")
        assert(#cfg:genericSettings() == 3)
        assert(cfg:genericSettings()[3].value == "4")

        cfg:setGenericSettings("no-sound", nil)
        assert(#cfg:genericSettings() == 0)

        assert(#cfg:contentFiles() == 0)
        assert(#cfg:fallbackArchives() == 0)
        assert(#cfg:dataDirectories() == 0)
        assert(#cfg:gameSettings() == 0)
        assert(cfg:userData() == nil)
        assert(cfg:resources() == nil)
        assert(cfg:dataLocal() == nil)
        assert(cfg:encoding() == nil)

        cfg:addContentFile("LuaMod.esp")
        cfg:addDataDirectory(dataDir)
        cfg:setGameSetting("fJumpHeight,1.0", nil, nil)

        -- A list argument must hold strings.
        local ok, message = pcall(function() cfg:setContentFiles({"ok", 5}) end)
        assert(not ok and string.find(message, "list entry 2", 1, true), message)

        cfg:saveUser()
    "#,
    );

    let saved = std::fs::read_to_string(root.join("openmw.cfg")).unwrap();
    assert!(saved.contains("content=LuaMod.esp"));
    assert!(saved.contains(&format!("data={}", data_dir.display())));
    assert!(saved.contains("fallback=fJumpHeight,1.0"));
}

#[test]
fn generic_settings_and_save_to_path() {
    let root = temp_dir("generic_save_root");
    write_cfg(&root, "no-sound=1\n");
    let out = temp_dir("generic_save_out").join("imported-openmw.cfg");

    let runtime = runtime(&[
        ("rootPath", root.display().to_string()),
        ("outPath", out.display().to_string()),
    ]);
    exec(
        &runtime,
        r#"
        local cfg = openmwConfig.new(rootPath)

        cfg:setGenericSettings("no-sound", {"0", "1"})
        cfg:addGenericSetting("some-flag", "yes")

        local settings = cfg:genericSettings()
        assert(#settings == 3)
        assert(settings[1].key == "no-sound")
        assert(settings[1].value == "0")
        assert(settings[2].value == "1")
        assert(settings[3].key == "some-flag")
        local plain = settings:toTable()
        assert(#plain == 3 and plain[3].value == "yes")

        cfg:saveToPath(outPath)
    "#,
    );

    let saved = std::fs::read_to_string(&out).unwrap();
    assert!(saved.contains("no-sound=0"));
    assert!(saved.contains("no-sound=1"));
    assert!(saved.contains("some-flag=yes"));
}

#[test]
fn empty_optional_and_resolved_export_surface() {
    let missing_dir = temp_dir("empty_optional_missing_root").join("future-config");
    let existing_empty_dir = temp_dir("empty_optional_existing_empty_root");
    let existing_root = temp_dir("empty_optional_existing_root");
    let out = temp_dir("resolved_save_out").join("openmw.cfg");
    write_cfg(
        &existing_root,
        "data=Data Files\nconfig=unused\ncontent=Existing.esm\n",
    );

    let runtime = runtime(&[
        ("missingDir", missing_dir.display().to_string()),
        ("existingEmptyDir", existing_empty_dir.display().to_string()),
        ("existingRoot", existing_root.display().to_string()),
        ("outPath", out.display().to_string()),
    ]);
    exec(
        &runtime,
        r#"
        local empty = openmwConfig.newEmpty(missingDir)
        assert(string.find(empty:rootConfigFile(), "openmw.cfg") ~= nil)
        assert(empty.isUserConfig == true)
        empty:setUserData("user-data")
        empty:setResources("resources")
        empty:setDataLocal("local-data")
        assert(string.find(empty:toString(), "user-data=user-data", 1, true) ~= nil)

        local emptyExisting = openmwConfig.loadOptional(existingEmptyDir)
        assert(string.find(emptyExisting:rootConfigFile(), existingEmptyDir, 1, true) ~= nil)

        local cfg = openmwConfig.loadOptional(existingRoot)
        local resolved = cfg:toResolvedString()
        assert(string.find(resolved, "data=Data Files", 1, true) == nil)
        assert(string.find(resolved, "config=", 1, true) == nil)
        assert(string.find(resolved, "content=Existing.esm", 1, true) ~= nil)
        cfg:saveResolvedToPath(outPath)
    "#,
    );

    let saved = std::fs::read_to_string(out).unwrap();
    assert!(!saved.contains("data=Data Files"));
    assert!(!saved.contains("config="));
    assert!(saved.contains("content=Existing.esm"));
}

#[test]
fn save_subconfig_success() {
    let root = temp_dir("save_subconfig_root");
    let sub = temp_dir("save_subconfig_sub");
    write_cfg(&root, &format!("config={}\n", sub.display()));
    write_cfg(&sub, "content=Sub.esm\n");

    let runtime = runtime(&[
        ("rootPath", root.display().to_string()),
        ("subPath", sub.display().to_string()),
    ]);
    exec(
        &runtime,
        r#"
        local cfg = openmwConfig.new(rootPath)
        cfg:addContentFile("RootLocal.esp")
        cfg:saveSubconfig(subPath)
    "#,
    );

    let saved = std::fs::read_to_string(sub.join("openmw.cfg")).unwrap();
    assert!(saved.contains("content=RootLocal.esp"));
}

#[test]
fn error_surface_through_pcall() {
    let root = temp_dir("pcall_errors_root");
    let other = temp_dir("pcall_errors_other");
    write_cfg(&root, "content=Morrowind.esm\n");
    write_cfg(&other, "content=Other.esm\n");

    let runtime = runtime(&[
        ("rootPath", root.display().to_string()),
        ("otherPath", other.display().to_string()),
    ]);
    exec(
        &runtime,
        r#"
        local cfg = openmwConfig.new(rootPath)

        local okA, errA = pcall(function() cfg:addContentFile("Morrowind.esm") end)
        assert(okA == false)
        assert(string.find(errA, "Morrowind.esm", 1, true), errA)

        local okB, errB = pcall(function() cfg:setEncoding("utf8") end)
        assert(okB == false)
        assert(errB ~= nil)

        local okC, errC = pcall(function() cfg:setGameSetting("invalid", nil, nil) end)
        assert(okC == false)
        assert(errC ~= nil)

        local okD, errD = pcall(function() cfg:setGameSettings({"invalid"}) end)
        assert(okD == false)
        assert(errD ~= nil)

        local okE, errE = pcall(function() cfg:saveSubconfig(otherPath) end)
        assert(okE == false)
        assert(errE ~= nil)

        local okF, errF = pcall(function() cfg:hasContentFile(5) end)
        assert(okF == false and string.find(errF, "string", 1, true), errF)

        local okG, errG = pcall(function() openmwConfig.new("/nonexistent/totally/fake/path") end)
        assert(okG == false and errG ~= nil)
    "#,
    );
}

/// The gate every extension crate runs: the plan's `.d.luau` type checks in Luau's frontend,
/// and a strict script that requires the module by its canonical path has no diagnostics.
#[cfg(feature = "luau-analysis")]
#[test]
fn the_declared_types_check_and_a_strict_script_passes() {
    use l3i::analysis::{
        Analysis, AnalysisOptions, Definitions, Mode, ModuleConfig, SourceCode, SourceProvider,
    };

    struct Script(&'static str);
    impl SourceProvider for Script {
        fn read_source(&self, name: &str) -> Option<SourceCode> {
            (name == "config_script").then(|| SourceCode {
                text: self.0.to_owned(),
                is_script: true,
            })
        }
        fn module_config(&self, _: &str) -> ModuleConfig {
            ModuleConfig {
                mode: Mode::Strict,
                ..ModuleConfig::default()
            }
        }
    }

    const SCRIPT: &str = "--!strict\n\
        local openmwConfig = require('@dream/openmw-config')\n\
        local cfg: dream_openmw_Config = openmwConfig.newEmpty('config')\n\
        cfg:addContentFile('Morrowind.esm')\n\
        cfg:setDataDirectories({ 'Data Files' })\n\
        local content: dream_openmw_Strings = cfg:contentFiles()\n\
        local total: number = #content\n\
        local first: string? = content[1]\n\
        local joined = ''\n\
        for i, name in content do local n: string = name joined ..= n total += i end\n\
        local names: { string } = content:toTable()\n\
        local user: boolean = cfg.isUserConfig\n\
        local count: number = cfg.contentFileCount + cfg.gameSettingCount\n\
        local setting = cfg:getGameSetting('iMaxLevel')\n\
        if setting then print(setting.kind, setting.value, setting.source, setting.typed) end\n\
        local rows: dream_openmw_GameSettings = cfg:gameSettings()\n\
        local row: dream_openmw_GameSetting? = rows[1]\n\
        local kinds = ''\n\
        for _, r in rows do local kind: string = r.kind kinds ..= kind end\n\
        local chain: dream_openmw_ConfigChain = cfg:configChain()\n\
        local depth: number = 0\n\
        for _, entry in chain do local status: string = entry.status depth += entry.depth end\n\
        local generic: dream_openmw_GenericSettings = cfg:genericSettings()\n\
        local plain: { dream_openmw_GenericSetting } = generic:toTable()\n\
        local defaultPath, pathError = openmwConfig.tryDefaultConfigPath()\n\
        print(total, first, joined, names, user, count, #rows, row, kinds, #chain, depth)\n\
        print(#generic, #plain, defaultPath or pathError, openmwConfig.version)\n";

    let plan = plan();
    plan.check_definitions().unwrap();
    let definitions = plan.type_definitions();
    assert!(
        definitions.contains("declare extern type dream_openmw_Config with"),
        "{definitions}"
    );
    // The views name their element type: `#`, `[i]`, `for`, and `toTable` are typed with it.
    for typed in [
        "    [number]: string?",
        "    function toTable(self): { string }",
        "    [number]: dream_openmw_GameSetting?",
        "(number?, dream_openmw_GameSetting), {}, number)",
        "(number?, dream_openmw_ChainEntry), {}, number)",
        "    function toTable(self): { dream_openmw_GenericSetting }",
    ] {
        assert!(
            definitions.contains(typed),
            "{typed:?} missing:\n{definitions}"
        );
    }
    for fallback in [
        "(self, ...any): any",
        "(...any) -> ...any",
        ": any,\n",
        ": any\n",
    ] {
        assert!(
            !definitions.contains(fallback),
            "every member is typed ({fallback:?} found):\n{definitions}"
        );
    }
    let options = AnalysisOptions {
        definitions: vec![Definitions {
            name: "openmw-config.d.luau".to_owned(),
            source: definitions.clone(),
        }],
        ..Default::default()
    };
    let analysis = Analysis::new(plan.analysis_sources(Script(SCRIPT)), options)
        .unwrap_or_else(|error| panic!("{error}\n---\n{definitions}"));
    let report = analysis.check("config_script", false);
    let text: Vec<String> = report
        .diagnostics
        .iter()
        .map(|d| {
            format!(
                "config_script:{}:{}: {} ({:?})",
                d.span.begin_line + 1,
                d.span.begin_column + 1,
                d.text,
                d.kind
            )
        })
        .collect();
    assert!(report.is_clean(), "{}\n---\n{definitions}", text.join("\n"));
}

#[test]
fn the_plan_tags_the_hot_types_only() {
    let plan = plan();
    let tagged: Vec<&str> = plan
        .userdata()
        .iter()
        .filter(|u| u.owner == luau::EXTENSION_ID && u.tag.is_some())
        .map(|u| u.key.as_str())
        .collect();
    assert_eq!(
        tagged,
        [
            luau::CONFIG_TYPE,
            luau::GAME_SETTING_TYPE,
            luau::GAME_SETTINGS_TYPE,
            luau::STRINGS_TYPE
        ]
    );
    for key in [
        luau::GENERIC_SETTING_TYPE,
        luau::GENERIC_SETTINGS_TYPE,
        luau::CHAIN_ENTRY_TYPE,
        luau::CONFIG_CHAIN_TYPE,
    ] {
        assert_eq!(plan.tag_of(key), None, "{key} stays untagged");
    }
    assert!(
        plan.userdata_by_key(luau::CONFIG_TYPE)
            .unwrap()
            .member("isUserConfig")
            .is_some()
    );
}
