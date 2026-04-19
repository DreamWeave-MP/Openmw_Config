#[cfg(feature = "lua")]
mod lua_tests {
    use mlua::Lua;
    use openmw_config::create_lua_module;
    use std::{path::Path, sync::Mutex};

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write_cfg(dir: &Path, contents: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("openmw.cfg"), contents).unwrap();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let base =
            std::env::temp_dir().join(format!("openmw_cfg_lua_{name}_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn test_lua_module_exports_and_default_helpers() {
        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();

        lua.load(
            r#"
            assert(type(openmwConfig.version) == "string")

            assert(type(openmwConfig.defaultConfigPath()) == "string")
            assert(type(openmwConfig.defaultUserDataPath()) == "string")
            assert(type(openmwConfig.defaultDataLocalPath()) == "string")
            assert(type(openmwConfig.defaultLocalPath()) == "string")

            if openmwConfig.tryDefaultGlobalPath then
              local globalPath, globalErr = openmwConfig.tryDefaultGlobalPath()
              assert((globalPath ~= nil and globalErr == nil) or (globalPath == nil and globalErr ~= nil))
            end

            local cfgPath, cfgErr = openmwConfig.tryDefaultConfigPath()
            assert((cfgPath ~= nil and cfgErr == nil) or (cfgPath == nil and cfgErr ~= nil))

            local dataPath, dataErr = openmwConfig.tryDefaultUserDataPath()
            assert((dataPath ~= nil and dataErr == nil) or (dataPath == nil and dataErr ~= nil))

            local localPath, localErr = openmwConfig.tryDefaultLocalPath()
            assert((localPath ~= nil and localErr == nil) or (localPath == nil and localErr ~= nil))
        "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn test_lua_from_env_loader() {
        let _guard = ENV_LOCK.lock().unwrap();
        let root = temp_dir("from_env_root");
        write_cfg(&root, "content=FromEnv.esm\n");
        let root_cfg = root.join("openmw.cfg");

        // SAFETY: guarded by a global mutex so no concurrent env mutation occurs in tests.
        unsafe {
            std::env::set_var("OPENMW_CONFIG", &root_cfg);
        }

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.fromEnv()
            assert(cfg:hasContentFile("FromEnv.esm"))
        "#,
        )
        .exec()
        .unwrap();

        // SAFETY: guarded by a global mutex so no concurrent env mutation occurs in tests.
        unsafe {
            std::env::remove_var("OPENMW_CONFIG");
        }
    }

    #[test]
    fn test_lua_read_surface_comprehensive() {
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
                "content=Root.esm\ngroundcover=RootGrass.esp\nfallback-archive=Root.bsa\nencoding=win1252\nuser-data={}\nresources={}\ndata-local={}\ndata={}\nfallback=iDifficulty,20\nfallback=fScale,1.5\nfallback=sName,Hello\nconfig={}\nconfig={}\n",
                userdata_dir.display(),
                resources_dir.display(),
                data_local_dir.display(),
                data_dir.display(),
                sub.display(),
                missing.display()
            ),
        );
        write_cfg(&sub, "content=Sub.esm\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();
        lua.globals()
            .set("expectedUserData", userdata_dir.display().to_string())
            .unwrap();
        lua.globals()
            .set("expectedResources", resources_dir.display().to_string())
            .unwrap();
        lua.globals()
            .set("expectedDataLocal", data_local_dir.display().to_string())
            .unwrap();
        lua.globals()
            .set("expectedDataDir", data_dir.display().to_string())
            .unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.new(rootPath)

            assert(type(cfg:rootConfigFile()) == "string")
            assert(type(cfg:rootConfigDir()) == "string")
            assert(type(cfg:isUserConfig()) == "boolean")
            assert(type(cfg:userConfigPath()) == "string")
            assert(type(cfg:userConfig():toString()) == "string")

            assert(cfg:hasContentFile("Root.esm"))
            assert(cfg:hasContentFile("Sub.esm"))
            assert(cfg:hasGroundcoverFile("RootGrass.esp"))
            assert(cfg:hasArchiveFile("Root.bsa"))
            assert(cfg:hasDataDir(expectedDataDir))

            local subConfigs = cfg:subConfigs()
            assert(#subConfigs == 1)

            local chain = cfg:configChain()
            assert(#chain == 3)
            assert(chain[1].status == "loaded")
            assert(chain[2].status == "skippedMissing")
            assert(chain[3].status == "loaded")
            assert(type(chain[1].depth) == "number")
            assert(type(chain[1].path) == "string")

            local content = cfg:contentFiles()
            assert(#content == 2)
            local ground = cfg:groundcoverFiles()
            assert(#ground == 1)
            local archives = cfg:fallbackArchives()
            assert(#archives == 1)
            local dirs = cfg:dataDirectories()
            assert(#dirs >= 1)

            assert(string.find(cfg:userData(), expectedUserData) ~= nil)
            assert(string.find(cfg:resources(), expectedResources) ~= nil)
            assert(string.find(cfg:dataLocal(), expectedDataLocal) ~= nil)
            assert(cfg:encoding() == "win1252")

            local settings = cfg:gameSettings()
            assert(#settings == 3)
            assert(type(settings[1].key) == "string")
            assert(type(settings[1].value) == "string")
            assert(type(settings[1].kind) == "string")

            local game = cfg:getGameSetting("iDifficulty")
            assert(game ~= nil)
            assert(game.key == "iDifficulty")
            assert(game.value == "20")
            assert(game.kind == "Int")
            assert(cfg:getGameSetting("does.not.exist") == nil)

            assert(type(cfg:toString()) == "string")
        "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn test_lua_mutation_surface_and_persistence() {
        let root = temp_dir("mutate_surface_root");
        let data_dir = temp_dir("mutate_surface_data");
        let user_dir = temp_dir("mutate_surface_userdata");
        let resources_dir = temp_dir("mutate_surface_resources");
        let data_local_dir = temp_dir("mutate_surface_data_local");
        write_cfg(&root, "content=Morrowind.esm\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();
        lua.globals()
            .set("dataDir", data_dir.display().to_string())
            .unwrap();
        lua.globals()
            .set("userDir", user_dir.display().to_string())
            .unwrap();
        lua.globals()
            .set("resourcesDir", resources_dir.display().to_string())
            .unwrap();
        lua.globals()
            .set("dataLocalDir", data_local_dir.display().to_string())
            .unwrap();

        lua.load(
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

            cfg:saveUser()
        "#,
        )
        .exec()
        .unwrap();

        let saved = std::fs::read_to_string(root.join("openmw.cfg")).unwrap();
        assert!(saved.contains("content=LuaMod.esp"));
        assert!(saved.contains(&format!("data={}", data_dir.display())));
        assert!(saved.contains("fallback=fJumpHeight,1.0"));
    }

    #[test]
    fn test_lua_save_subconfig_success() {
        let root = temp_dir("save_subconfig_root");
        let sub = temp_dir("save_subconfig_sub");
        write_cfg(&root, &format!("config={}\n", sub.display()));
        write_cfg(&sub, "content=Sub.esm\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();
        lua.globals()
            .set("subPath", sub.display().to_string())
            .unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.new(rootPath)
            cfg:addContentFile("RootLocal.esp")
            cfg:saveSubconfig(subPath)
        "#,
        )
        .exec()
        .unwrap();

        let saved = std::fs::read_to_string(sub.join("openmw.cfg")).unwrap();
        assert!(saved.contains("content=RootLocal.esp"));
    }

    #[test]
    fn test_lua_error_surface_through_pcall() {
        let root = temp_dir("pcall_errors_root");
        let other = temp_dir("pcall_errors_other");
        write_cfg(&root, "content=Morrowind.esm\n");
        write_cfg(&other, "content=Other.esm\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();
        lua.globals()
            .set("otherPath", other.display().to_string())
            .unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.new(rootPath)

            local okA, errA = pcall(function() cfg:addContentFile("Morrowind.esm") end)
            assert(okA == false)
            assert(errA ~= nil)

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
        "#,
        )
        .exec()
        .unwrap();
    }
}
