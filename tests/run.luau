--!strict
--!native
local configuration = require '@openmw-config'
local fs = require '@dream/fs'
local test = require '@jess/test'

local expect = test.expect
local suite = test.new()
local scratch = assert(rawget(_G, 'scratchRoot') :: string?, 'the suite needs a host-owned scratchRoot')

local function io<Value>(value: Value?, message: string?, kind: dream_fs_ErrorKind?): Value
  if not value then error(message or tostring(kind), 0) end

  return value
end

local function fixture(name: string, source: string): (string, string)
  local directory = jess.fs.join(scratch, 'openmw-config', name)
  io(fs.mkdir(directory, { recursive = true }))
  local path = jess.fs.join(directory, 'openmw.cfg')
  io(fs.writeFile(path, source))
  return directory, path
end

local function read(path: string): string return io(fs.readFileString(path)) end

suite.describe('configuration ownership', function()
  suite.it('preserves untouched user bytes without creating configured directories', function()
    local source = '# untouched\r\n  encoding = win1252\r\nconfig="missing/user"\r\n'
    local directory, path = fixture('inspect', source)
    local config = configuration.new(path)
    expect(config:renderUser()).toBe ''
    expect(config:last('encoding')).toBe 'win1252'
    expect((fs.exists(jess.fs.join(directory, 'missing/user')))).toBe(false)
    expect(read(path)).toBe(source)
    expect(config:chain()[2].status).toBe 'skippedMissing'

    local _, standalone = fixture('untouched', (source:gsub('config="missing/user"\r\n', '')))
    expect(configuration.new(standalone):renderUser()).toBe(read(standalone))
  end)

  suite.it('loads depth first and retains traversal history after replace=config', function()
    local directory, path = fixture('chain', 'content=Root.esm\nconfig=a\nconfig=b\n')
    io(fs.mkdir(jess.fs.join(directory, 'a/c'), { recursive = true }))
    io(fs.mkdir(jess.fs.join(directory, 'b'), { recursive = true }))
    io(fs.writeFile(jess.fs.join(directory, 'a/openmw.cfg'), 'content=A.esp\nconfig=c\n'))
    io(fs.writeFile(jess.fs.join(directory, 'a/c/openmw.cfg'), 'content=C.esp\n'))
    io(fs.writeFile(jess.fs.join(directory, 'b/openmw.cfg'), 'replace=config\ncontent=B.esp\n'))
    local config = configuration.new(path)
    expect(config:values('content')).toEqual { 'Root.esm', 'B.esp' }
    expect(#config:chain()).toBe(4)
    expect(config:userFile()).toBe(jess.fs.join(directory, 'b/openmw.cfg'))
  end)

  suite.it('persists inherited removal without touching the parent', function()
    local source = '# parent\ncontent=Keep.esm\ncontent=Remove.esp\nconfig=user\n'
    local directory, path = fixture('removal', source)
    io(fs.mkdir(jess.fs.join(directory, 'user'), { recursive = true }))
    local user = jess.fs.join(directory, 'user/openmw.cfg')
    io(fs.writeFile(user, '# user\ncontent=Mine.esp\n# trailer\n'))
    local config = configuration.new(path)
    expect(config:remove('content', 'Remove.esp')).toBe(true)
    expect(config:renderUser()).toBe 'replace=content\ncontent=Keep.esm\n# user\ncontent=Mine.esp\n# trailer\n'
    config:saveUser()
    expect(read(path)).toBe(source)
    expect(configuration.new(path):values('content')).toEqual { 'Keep.esm', 'Mine.esp' }
  end)

  suite.it('relocates inherited relative directories by provenance, not value equality', function()
    local directory, path = fixture('relative-data', 'data=first\ndata=second\nresources=resources\ndata-local=second\nconfig=user\n')
    io(fs.mkdir(jess.fs.join(directory, 'user'), { recursive = true }))
    io(fs.writeFile(jess.fs.join(directory, 'user/openmw.cfg'), ''))
    local config = configuration.new(path)
    expect(#config:dataDirectories()).toBe(4)
    config:remove('data', 'first')
    config:saveUser()
    local reloaded = configuration.new(path)
    expect(reloaded:values('data')).toEqual { jess.fs.join(directory, 'second') }
    expect(#reloaded:dataDirectories()).toBe(3)
    expect(reloaded:dataDirectories()[2]).toBe(reloaded:dataDirectories()[3])
  end)

  suite.it('validates complete replacements before changing state', function()
    local directory = fixture('transactional', '')
    local config = configuration.newEmpty(directory)
    config:append('content', 'Keep.esm')
    local before = config:generation()
    expect(function() config:replace('content', { 'New.esp', 'New.esp' }) end).toThrow 'duplicate content'
    expect(function() config:replace('fallback', { 'Valid,1', 'not-a-pair' }) end).toThrow 'fallback needs'
    expect(function() config:append('content', 'injected\nconfig=elsewhere') end).toThrow 'one key=value line'
    expect(function() config:append('#hidden', 'value') end).toThrow 'one key=value line'
    expect(config:values('content')).toEqual { 'Keep.esm' }
    expect(config:generation()).toBe(before)
  end)

  suite.it('refuses to overwrite an unmodeled sibling of a custom config', function()
    local directory, user = fixture('custom-save', 'content=Valuable.esm\n')
    local custom = jess.fs.join(directory, 'custom.cfg')
    io(fs.writeFile(custom, 'content=Custom.esm\n'))
    local config = configuration.new(custom)
    config:append('content', 'New.esp')
    expect(function() config:saveUser() end).toThrow 'unmodeled user file'
    expect(read(user)).toBe 'content=Valuable.esm\n'
    expect(read(custom)).toBe 'content=Custom.esm\n'
    io(fs.remove(user))
    expect(function() configuration.new(custom):saveUser() end).toThrow 'unmodeled user file'
    expect((fs.exists(user))).toBe(false)
  end)

  suite.it('detects external changes and advances its baseline after successful saves', function()
    local source = 'content=Base.esm\n'
    local directory, path = fixture('save-conflict', source)
    local config = configuration.new(path)
    io(fs.writeFile(path, 'content=External.esm\n'))
    expect(function() config:saveUser({ backup = true }) end).toThrow 'changed since it was loaded'
    expect(read(path)).toBe 'content=External.esm\n'
    expect((fs.exists(path .. '.bak'))).toBe(false)
    io(fs.writeFile(path, source))
    config:append('content', 'First.esp')
    config:saveUser()
    config:append('content', 'Second.esp')
    config:saveUser()
    expect(configuration.new(path):values('content')).toEqual { 'Base.esm', 'First.esp', 'Second.esp' }

    local empty = configuration.newEmpty(jess.fs.join(directory, 'new-user'))
    io(fs.mkdir(jess.fs.join(directory, 'new-user'), { recursive = true }))
    io(fs.writeFile(empty:userFile(), 'content=Appeared.esm\n'))
    expect(function() empty:saveUser() end).toThrow 'changed since it was loaded'
    expect(read(empty:userFile())).toBe 'content=Appeared.esm\n'
  end)

  suite.it('quotes relocated paths with significant whitespace and escape characters', function()
    local directory, path = fixture('quoted-relocation', 'data=remove\ndata="second &&&"name "\nconfig=user\n')
    io(fs.mkdir(jess.fs.join(directory, 'user'), { recursive = true }))
    io(fs.writeFile(jess.fs.join(directory, 'user/openmw.cfg'), ''))
    local config = configuration.new(path)
    config:remove('data', 'remove')
    local expected = config:dataDirectories()
    config:saveUser()
    expect(configuration.new(path):dataDirectories()).toEqual(expected)
    local exported = jess.fs.join(directory, 'export.cfg')
    io(fs.writeFile(exported, config:render(true)))
    expect(configuration.new(exported):dataDirectories()).toEqual(expected)
  end)

  suite.it('requires explicit roots for recognized tokens and creates nothing on failure', function()
    local directory, path = fixture('missing-token', 'config="?userconfig?"\n')
    expect(function() configuration.new(path) end).toThrow "needs an explicit 'userConfig' root"
    expect((fs.exists(jess.fs.join(directory, '?userconfig?')))).toBe(false)
  end)

  suite.it('clears only user singleton overrides, leaving parent definitions intact', function()
    local source = 'resources=parent-resources\nconfig=user\n'
    local directory, path = fixture('singleton-ownership', source)
    io(fs.mkdir(jess.fs.join(directory, 'user'), { recursive = true }))
    io(fs.writeFile(jess.fs.join(directory, 'user/openmw.cfg'), 'resources=user-resources\n'))
    local config = configuration.new(path)
    config:set('resources', nil)
    local parent = config:path('resources')
    config:set('resources', nil)
    expect(config:path('resources')).toBe(parent)
    config:saveUser()
    expect(configuration.new(path):path('resources')).toBe(parent)
    expect(read(path)).toBe(source)
  end)

  suite.it('keeps token roots separate from configured singleton overrides', function()
    local directory, path = fixture('tokens', 'user-data=override\ndata-local="?userdata?\\local"\ndata="?userconfig?/data"\n')
    local tokens: configuration.Tokens = { userData = jess.fs.join(directory, 'default-data'), userConfig = jess.fs.join(directory, 'default-config') }
    local config = configuration.new(path, { tokens = tokens })
    expect(config:path('user-data')).toBe(jess.fs.join(directory, 'override'))
    expect(config:path('data-local')).toBe(jess.fs.join(tokens.userData :: string, 'local'))
    expect(config:dataDirectories()[1]).toBe(jess.fs.join(tokens.userConfig :: string, 'data'))
  end)

  suite.it('resolves relative paths from any explicitly named config file', function()
    local directory = fixture('custom-name', '')
    local custom = jess.fs.join(directory, 'custom.cfg')
    io(fs.writeFile(custom, 'data=relative\n'))
    expect(configuration.new(custom):dataDirectories()).toEqual { jess.fs.join(directory, 'relative') }
  end)
end)

return suite
