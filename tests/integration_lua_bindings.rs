#[cfg(feature = "lua")]
mod lua_tests {
    use mlua::Lua;
    use openmw_config::create_lua_module;
    use std::path::Path;

    fn write_cfg(dir: &Path, contents: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("openmw.cfg"), contents).unwrap();
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "openmw_cfg_lua_{name}_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    #[test]
    fn test_lua_load_and_read_views() {
        let root = temp_dir("read_views_root");
        let sub = temp_dir("read_views_sub");

        write_cfg(
            &root,
            &format!(
                "content=Root.esm\nconfig={}\n",
                sub.display()
            ),
        );
        write_cfg(&sub, "content=Sub.esm\nfallback=iDifficulty,20\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.new(rootPath)
            assert(cfg:hasContentFile("Root.esm"))
            assert(cfg:hasContentFile("Sub.esm"))

            local chain = cfg:configChain()
            assert(#chain >= 2)
            assert(chain[1].status == "loaded")

            local game = cfg:getGameSetting("iDifficulty")
            assert(game ~= nil)
            assert(game.key == "iDifficulty")
            assert(game.value == "20")
            assert(type(cfg:toString()) == "string")
        "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn test_lua_mutate_and_save_user() {
        let root = temp_dir("mutate_save_root");
        write_cfg(&root, "content=Morrowind.esm\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.new(rootPath)
            cfg:addContentFile("LuaMod.esp")
            cfg:setDataDirectories({"/tmp/lua-data-dir"})
            cfg:setGameSetting("fJumpHeight,1.0", nil, nil)
            cfg:saveUser()
        "#,
        )
        .exec()
        .unwrap();

        let saved = std::fs::read_to_string(root.join("openmw.cfg")).unwrap();
        assert!(saved.contains("content=LuaMod.esp"));
        assert!(saved.contains("data=/tmp/lua-data-dir"));
        assert!(saved.contains("fallback=fJumpHeight,1.0"));
    }

    #[test]
    fn test_lua_duplicate_errors_surface_through_pcall() {
        let root = temp_dir("pcall_errors_root");
        write_cfg(&root, "content=Morrowind.esm\n");

        let lua = Lua::new();
        let module = create_lua_module(&lua).unwrap();
        lua.globals().set("openmwConfig", module).unwrap();
        lua.globals()
            .set("rootPath", root.display().to_string())
            .unwrap();

        lua.load(
            r#"
            local cfg = openmwConfig.new(rootPath)
            local ok, err = pcall(function()
                cfg:addContentFile("Morrowind.esm")
            end)
            assert(ok == false)
            assert(err ~= nil)
        "#,
        )
        .exec()
        .unwrap();
    }
}
