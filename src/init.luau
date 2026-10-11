--!strict
--!native
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- The configuration chain owns source attribution. Inspection never creates directories; saving
-- the user file never rewrites its parents.
local fs = require '@dream/fs'
local paths = require '@self/paths'
local text = require '@self/text'

export type Entry = text.Entry
export type Tokens = paths.Tokens
export type Options = { tokens: Tokens? }
export type ChainEntry = { path: string, depth: number, status: 'loaded' | 'skippedMissing' }
export type Written = { path: string, backup: string? }
export type Config = {
  read values: (self: Config, key: string) -> { string },
  read last: (self: Config, key: string) -> string?,
  read path: (self: Config, key: string) -> string?,
  read dataDirectories: (self: Config) -> { string },
  read entries: (self: Config) -> { Entry },
  read chain: (self: Config) -> { ChainEntry },
  read rootFile: (self: Config) -> string,
  read userFile: (self: Config) -> string,
  read generation: (self: Config) -> number,
  read append: (self: Config, key: string, value: string) -> (),
  read remove: (self: Config, key: string, value: string) -> boolean,
  read replace: (self: Config, key: string, values: { string }?) -> (),
  read set: (self: Config, key: string, value: string?) -> (),
  read render: (self: Config, resolved: boolean?) -> string,
  read renderUser: (self: Config) -> string,
  read saveUser: (self: Config, options: { backup: boolean? }?) -> Written,
}

type File = { entries: { Entry } }
type State = {
  entries: { Entry },
  chain: { ChainEntry },
  originals: { [string]: string },
  owned: { [string]: boolean },
  ownsUserFile: boolean,
  root: string,
  user: string,
  tokens: Tokens,
  replaces: { string },
  generation: number,
}

local states = (setmetatable({}, { __mode = 'k' }) :: unknown) :: { [Config]: State }
local Methods = {}

local function stateOf(config: Config): State
  local state = states[config]
  assert(state, 'dreamwind.openmw-config: invalid configuration receiver')
  return state
end

local function stat(path: string): dream_fs_Stat?
  local result, message = fs.stat(path)
  if message then error(message, 0) end

  return result
end

local function options(given: Options?): Tokens
  local selected: Options = given or {}
  for name in (selected :: unknown) :: { [string]: unknown } do
    assert(name == 'tokens', `dreamwind.openmw-config: unknown option '{name}'`)
  end

  local input: Tokens = selected.tokens or {}
  local tokens = table.clone(input)
  for name, value in (tokens :: unknown) :: { [string]: unknown } do
    assert(name == 'localPath' or name == 'globalPath' or name == 'userConfig' or name == 'userData',
      `dreamwind.openmw-config: unknown path token '{name}'`)
    assert(type(value) == 'string', `dreamwind.openmw-config: token '{name}' needs a path string`)
  end

  return tokens
end

local function resolved(entry: Entry, tokens: Tokens): Entry
  if entry.key and text.DirectoryKeys[entry.key] then
    entry.resolved = paths.resolve(entry.value :: string, entry.source, tokens)
  end

  return entry
end

local function handle(state: State): Config
  -- Keep behavior on the handle so generic serializers reject it, rather than silently saving {}.
  local config = (table.clone(Methods) :: unknown) :: Config
  states[config] = state
  table.freeze(config)
  return config
end

local function empty(directory: string, tokens: Tokens): Config
  assert(directory ~= '' and jess.fs.fileName(directory) ~= 'openmw.cfg',
    'dreamwind.openmw-config: newEmpty needs a configuration directory')
  local file = jess.fs.join(paths.absolute(directory), 'openmw.cfg')
  return handle {
    entries = {}, chain = {}, originals = {}, owned = { [file] = true }, root = file, user = file,
    ownsUserFile = true, tokens = tokens, replaces = {}, generation = 0,
  }
end

local function load(path: string, tokens: Tokens): Config
  assert(path ~= '', 'dreamwind.openmw-config: configuration path is empty')
  local root = paths.absolute(path)
  local rootInfo = stat(root)
  if rootInfo and rootInfo.isDir then root = jess.fs.join(root, 'openmw.cfg') end

  local pending = { { path = root, depth = 0 } }
  local tried: { [string]: boolean } = {}
  local files: { File }, active: { string }, chain: { ChainEntry } = {}, {}, {}
  local originals: { [string]: string } = {}
  while #pending > 0 do
    local nextConfig = table.remove(pending) :: { path: string, depth: number }
    local file, depth = nextConfig.path, nextConfig.depth
    local directory = jess.fs.parent(file) :: string
    local identity = paths.key(directory)
    if tried[identity] then continue end

    tried[identity] = true
    local info = stat(file)
    if not info then
      if depth == 0 then text.fail(file, 0, 'configuration file does not exist') end

      local event: ChainEntry = { path = file, depth = depth, status = 'skippedMissing' }
      table.insert(chain, event)
      table.insert(active, directory)
      continue
    end

    if depth > 16 then text.fail(file, 0, 'configuration chain exceeds depth 16') end

    local source, message = fs.readFileString(file)
    if not source then error(message, 0) end

    local entries = text.parse(source, file)
    for _, entry in entries do resolved(entry, tokens) end

    local replacesConfig = false
    for _, entry in entries do
      if entry.key == 'replace' and entry.value == 'config' then replacesConfig = true end
    end

    if #files > 1 and replacesConfig then
      files = { files[1] }
      active = { active[1] }
    end

    for index = #entries, 1, -1 do
      local entry = entries[index]
      if entry.key == 'config' then
        table.insert(pending, { path = jess.fs.join(entry.resolved :: string, 'openmw.cfg'), depth = depth + 1 })
      end
    end

    originals[file] = source
    local event: ChainEntry = { path = file, depth = depth, status = 'loaded' }
    table.insert(chain, event)
    table.insert(active, directory)
    table.insert(files, { entries = entries })
  end

  local activeSet: { [string]: boolean } = {}
  for _, directory in active do activeSet[paths.key(directory)] = true end

  local replaced: { [string]: boolean } = {}
  local kept: { { Entry } } = {}
  for index = #files, 1, -1 do
    local entries: { Entry } = {}
    for _, entry in files[index].entries do
      local key = entry.key
      if key == 'config' then
        if activeSet[paths.key(entry.resolved :: string)] then table.insert(entries, entry) end
      elseif not key or text.SingletonKeys[key] or not replaced[key] then
        table.insert(entries, entry)
      end
    end

    for _, entry in entries do
      if entry.key == 'replace' then replaced[entry.value :: string] = true end
    end

    table.insert(kept, entries)
  end

  local effective: { Entry } = {}
  local seen: { [string]: { [string]: boolean } } = {}
  for index = #kept, 1, -1 do
    for _, entry in kept[index] do
      local key = entry.key
      if key and text.UniqueKeys[key] then
        local values = seen[key] or {}
        if values[entry.value :: string] then text.fail(entry.source, entry.line, `duplicate {key} '{entry.value}'`) end

        values[entry.value :: string] = true
        seen[key] = values
      end

      table.insert(effective, entry)
    end
  end

  local user = jess.fs.join(active[#active], 'openmw.cfg')
  local ownsUserFile = false
  for _, event in chain do
    if paths.key(event.path) == paths.key(user) then ownsUserFile = true end
  end

  local owned: { [string]: boolean } = { [user] = true }
  local userOriginal = originals[user]
  for file in originals do
    local same = paths.key(file) == paths.key(user)
    if not same then
      local equivalent, message = fs.sameFile(file, user)
      if message then error(message, 0) end

      same = equivalent == true
    end

    owned[file] = same
    if same then
      ownsUserFile = true
      userOriginal = userOriginal or originals[file]
    end
  end

  originals[user] = userOriginal

  return handle {
    entries = effective, chain = chain, originals = originals, owned = owned, root = root,
    user = user, ownsUserFile = ownsUserFile, tokens = tokens, replaces = {}, generation = 0,
  }
end

local function new(path: string, given: Options?): Config return load(path, options(given)) end

local function newEmpty(directory: string, given: Options?): Config return empty(directory, options(given)) end

local function loadOptional(path: string, given: Options?): Config
  assert(path ~= '', 'dreamwind.openmw-config: configuration path is empty')
  local tokens = options(given)
  local absolute = paths.absolute(path)
  local info = stat(absolute)
  if info and not info.isDir then return load(absolute, tokens) end

  if info and stat(jess.fs.join(absolute, 'openmw.cfg')) then return load(absolute, tokens) end

  local directory = if jess.fs.fileName(absolute) == 'openmw.cfg'
    then jess.fs.parent(absolute) :: string else absolute
  return empty(directory, tokens)
end

function Methods.values(self: Config, key: string): { string }
  local values: { string } = {}
  for _, entry in stateOf(self).entries do
    if entry.key == key then table.insert(values, entry.value :: string) end
  end

  return values
end

function Methods.last(self: Config, key: string): string?
  local entries = stateOf(self).entries
  for index = #entries, 1, -1 do
    if entries[index].key == key then return entries[index].value end
  end

  return nil
end

function Methods.path(self: Config, key: string): string?
  assert(text.DirectoryKeys[key], `dreamwind.openmw-config: '{key}' is not a directory setting`)
  local entries = stateOf(self).entries
  for index = #entries, 1, -1 do
    if entries[index].key == key then return entries[index].resolved end
  end

  return nil
end

function Methods.dataDirectories(self: Config): { string }
  local result: { string } = {}
  local resources = self:path('resources')
  if resources then table.insert(result, jess.fs.join(resources, 'vfs')) end

  for _, entry in stateOf(self).entries do
    if entry.key == 'data' then table.insert(result, entry.resolved :: string) end
  end

  local dataLocal = self:path('data-local')
  if dataLocal then table.insert(result, dataLocal) end

  return result
end

function Methods.entries(self: Config): { Entry }
  local result: { Entry } = {}
  for _, entry in stateOf(self).entries do table.insert(result, table.clone(entry)) end

  return result
end

function Methods.chain(self: Config): { ChainEntry }
  local result: { ChainEntry } = {}
  for _, entry in stateOf(self).chain do table.insert(result, table.clone(entry)) end

  return result
end

function Methods.rootFile(self: Config): string return stateOf(self).root end

function Methods.userFile(self: Config): string return stateOf(self).user end

function Methods.generation(self: Config): number return stateOf(self).generation end

local function composing(key: string)
  assert(not text.SingletonKeys[key] and key ~= 'config' and key ~= 'replace',
    `dreamwind.openmw-config: '{key}' is not an editable composing list`)
end

local function replacement(state: State, key: string)
  if not table.find(state.replaces, key) then table.insert(state.replaces, key) end
end

local function entryFor(state: State, key: string, value: string): Entry
  text.validate(key, value, state.user, 0)
  return resolved({ key = key, value = value, source = state.user, line = 0, comment = '' }, state.tokens)
end

function Methods.append(self: Config, key: string, value: string)
  composing(key)
  local state = stateOf(self)
  local entry = entryFor(state, key, value)
  if text.UniqueKeys[key] then
    assert(not table.find(self:values(key), value), `dreamwind.openmw-config: duplicate {key} '{value}'`)
  end

  table.insert(state.entries, entry)
  state.generation += 1
end

function Methods.remove(self: Config, key: string, value: string): boolean
  composing(key)
  local state = stateOf(self)
  local kept: { Entry } = {}
  local changed = false
  for _, entry in state.entries do
    if entry.key == key and (entry.value == value or (entry.resolved and paths.key(entry.resolved) == paths.key(value))) then
      changed = true
      if not state.owned[entry.source] then replacement(state, key) end
    else
      table.insert(kept, entry)
    end
  end

  if changed then
    state.entries = kept
    state.generation += 1
  end

  return changed
end

function Methods.replace(self: Config, key: string, values: { string }?)
  composing(key)
  local state = stateOf(self)
  local additions: { Entry } = {}
  local seen: { [string]: boolean } = {}
  for _, value in values or {} do
    if text.UniqueKeys[key] and seen[value] then text.fail(state.user, 0, `duplicate {key} '{value}'`) end

    table.insert(additions, entryFor(state, key, value))
    seen[value] = true
  end

  local kept: { Entry } = {}
  for _, entry in state.entries do
    if entry.key ~= key then
      table.insert(kept, entry)
    elseif not state.owned[entry.source] then
      replacement(state, key)
    end
  end

  table.move(additions, 1, #additions, #kept + 1, kept)
  state.entries = kept
  state.generation += 1
end

function Methods.set(self: Config, key: string, value: string?)
  assert(text.SingletonKeys[key], `dreamwind.openmw-config: '{key}' is not a singleton setting`)
  assert(value == nil or type(value) == 'string', 'dreamwind.openmw-config: a singleton needs a string or nil')
  local state = stateOf(self)
  local addition = if value then entryFor(state, key, value) else nil
  for index = #state.entries, 1, -1 do
    local entry = state.entries[index]
    if entry.key ~= key then continue end

    if not addition then
      -- A user save cannot erase a parent's singleton; clearing only removes a user override.
      if not state.owned[entry.source] then return end

      table.remove(state.entries, index)
    elseif state.owned[entry.source] then
      state.entries[index] = addition
    else
      table.insert(state.entries, addition)
    end

    state.generation += 1
    return
  end

  if addition then
    table.insert(state.entries, addition)
    state.generation += 1
  end
end

function Methods.render(self: Config, resolvePaths: boolean?): string
  local result: { string }, trailing: { Entry } = {}, {}
  for _, entry in stateOf(self).entries do
    if resolvePaths then
      if entry.key ~= 'config' and (not entry.key or string.lower(entry.key) ~= 'replace') then
        table.insert(result, text.render(entry, true))
      end
    elseif not entry.key then
      table.insert(trailing, entry)
    else
      if trailing[1] and trailing[1].source ~= entry.source then
        for _, ending in trailing do table.insert(result, ending.comment) end

        table.clear(trailing)
      end

      table.insert(result, text.render(entry))
    end
  end

  for _, ending in trailing do table.insert(result, ending.comment) end

  return table.concat(result)
end

function Methods.renderUser(self: Config): string
  local state = stateOf(self)
  if state.generation == 0 then return state.originals[state.user] or '' end

  local result: { string }, trailing: { string } = {}, {}
  local pending = table.clone(state.replaces)
  local function inherited(key: string)
    for _, entry in state.entries do
      if entry.key ~= key or state.owned[entry.source] then continue end

      local copy = table.clone(entry)
      copy.comment = ''
      if copy.resolved and paths.key(paths.resolve(copy.value :: string, state.user, state.tokens)) ~= paths.key(copy.resolved) then
        copy.value = text.encodeDirectory(copy.resolved)
      end

      table.insert(result, text.render(copy))
    end
  end

  for _, entry in state.entries do
    if not state.owned[entry.source] then continue end

    if not entry.key then
      table.insert(trailing, entry.comment)
      continue
    end

    local key = if entry.key == 'replace' then entry.value :: string else entry.key
    local index = table.find(pending, key)
    if index then
      table.remove(pending, index)
      if entry.key == 'replace' then
        table.insert(result, text.render(entry))
      else
        table.insert(result, `replace={key}\n`)
      end

      inherited(key)
      if entry.key == 'replace' then continue end
    end

    table.insert(result, text.render(entry))
  end

  for _, key in pending do
    table.insert(result, `replace={key}\n`)
    inherited(key)
  end

  table.move(trailing, 1, #trailing, #result + 1, result)
  return table.concat(result)
end

function Methods.saveUser(self: Config, options: { backup: boolean? }?): Written
  local state = stateOf(self)
  assert(state.ownsUserFile, `dreamwind.openmw-config: refusing to save unmodeled user file '{state.user}'`)
  local selected: { backup: boolean? } = options or {}
  for name in (selected :: unknown) :: { [string]: unknown } do
    assert(name == 'backup', `dreamwind.openmw-config: unknown save option '{name}'`)
  end

  assert(selected.backup == nil or type(selected.backup) == 'boolean', 'dreamwind.openmw-config: backup needs a boolean')
  local current, problem, kind = fs.readFileString(state.user)
  if not current and kind ~= 'notFound' then error(problem, 0) end

  assert(current == state.originals[state.user],
    `dreamwind.openmw-config: user file '{state.user}' changed since it was loaded; reload before saving`)
  local output = self:renderUser()
  local created, message = fs.mkdir(jess.fs.parent(state.user) :: string, { recursive = true })
  assert(created, message)
  local backup: string?
  if selected.backup and current then
    local index = 0
    while true do
      local candidate = `{state.user}.bak{if index == 0 then '' else tostring(index)}`
      local exists, reason = fs.exists(candidate)
      if reason then error(reason, 0) end

      if not exists then
        local copied, copyError = fs.copy(state.user, candidate)
        assert(copied, copyError)
        backup = candidate
        break
      end

      index += 1
    end
  end

  -- The existing generic atomic-replace seam preserves permissions and failed-write cleanup.
  -- It will move to L3i before the legacy plugin crate that currently implements it is deleted.
  jess.fs.replace(state.user, output)
  state.originals[state.user] = output
  return { path = state.user, backup = backup }
end

table.freeze(Methods)

return table.freeze { new = new, newEmpty = newEmpty, loadOptional = loadOptional }
