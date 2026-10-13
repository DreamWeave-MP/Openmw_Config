--!strict
--!native
-- The grammar and the paths as they were first written, a pattern class for every test and a walk
-- for every character, kept here as the statement of what the shipped ones return and raise. Each
-- case runs both over every short spelling of an alphabet holding every byte either one treats
-- specially, and they have to agree on the result or on the text of the error.
local paths = require '@openmw-config/paths'
local test = require '@jess/test'
local text = require '@openmw-config/text'

type Entry = text.Entry
type Tokens = paths.Tokens

local expect = test.expect
local suite = test.new()

local Windows = string.find(jess.fs.join('first', 'second'), '\\', 1, true) ~= nil
local Separator = if Windows then '\\' else '/'
local Stamp = '# OpenMW-Config Serializer Version:'

local Oracle = {}

function Oracle.separators(path: string): string return (string.gsub(path, '[/\\]', Separator)) end

function Oracle.rooted(path: string): boolean
  return string.sub(path, 1, 1) == Separator
    or (Windows and string.match(path, '^%a:[/\\]') ~= nil)
end

function Oracle.key(path: string): string
  local normalized = Oracle.separators(path)
  local parts: { string } = {}

  for part in string.gmatch(normalized, '[^/\\]+') do
    if part ~= '.' then table.insert(parts, part) end
  end

  local prefix = if Windows and string.sub(normalized, 1, 2) == '\\\\'
    then '\\\\'
    elseif string.sub(normalized, 1, 1) == Separator then Separator else ''
  return prefix .. table.concat(parts, Separator)
end

function Oracle.unquote(value: string): string
  if string.sub(value, 1, 1) ~= '"' then return value end

  local result: { string } = {}
  local escaped = false

  for position = 2, #value do
    local character = string.sub(value, position, position)
    if escaped then
      table.insert(result, character)
      escaped = false
    elseif character == '&' then
      escaped = true
    elseif character == '"' then
      break
    else
      table.insert(result, character)
    end
  end

  return table.concat(result)
end

function Oracle.resolve(value: string, source: string, tokens: Tokens): string
  local path = Oracle.unquote(value)
  local roots: { [string]: string } = {
    ['?local?'] = 'localPath',
    ['?global?'] = 'globalPath',
    ['?userconfig?'] = 'userConfig',
    ['?userdata?'] = 'userData',
  }

  for token, field in roots do
    if string.sub(path, 1, #token) ~= token then continue end

    local root = ((tokens :: unknown) :: { [string]: string? })[field]
    assert(root, `openmw-config: {source}: token '{token}' needs an explicit '{field}' root`)
    local suffix = string.gsub(string.sub(path, #token + 1), '^[/\\]+', '')
    path = jess.fs.join(root, suffix)
    break
  end

  path = Oracle.separators(path)
  if Oracle.rooted(path) then return path end

  return jess.fs.join(jess.fs.parent(source) or '.', path)
end

function Oracle.fail(source: string, line: number, reason: string): never
  return error(`openmw-config: {source}:{line}: {reason}`, 0)
end

function Oracle.validate(key: string, value: string, source: string, line: number)
  if type(key) ~= 'string' or type(value) ~= 'string' then
    Oracle.fail(source, line, 'settings need string keys and values')
  end

  if not utf8.len(key) or not utf8.len(value) then Oracle.fail(source, line, 'settings must be UTF-8') end

  if key == '' or string.sub(key, 1, 1) == '#' or string.find(key, '[=\r\n%z]') or string.find(value, '[\r\n%z]') then
    Oracle.fail(source, line, 'a setting must fit on one key=value line without NUL bytes')
  end

  if text.trim(key) ~= key or text.trim(value) ~= value then
    Oracle.fail(source, line, 'setting keys and values must not have surrounding whitespace')
  end

  if key == 'encoding' and value ~= 'win1250' and value ~= 'win1251' and value ~= 'win1252' then
    Oracle.fail(source, line, `unsupported encoding '{value}'`)
  end

  if key == 'fallback' and not string.find(value, ',', 1, true) then
    Oracle.fail(source, line, `fallback needs Key,Value, got '{value}'`)
  end
end

function Oracle.encodeDirectory(value: string): string
  assert(utf8.len(value) and not string.find(value, '[\r\n%z]'),
    'openmw-config: directory paths must be single-line UTF-8 for export')
  if text.trim(value) == value and string.sub(value, 1, 1) ~= '"' then return value end

  return '"' .. string.gsub(value, '[&"]', function(character: string): string
    return '&' .. character
  end) .. '"'
end

function Oracle.lines(source: string): { string }
  local result: { string } = {}
  local start = 1

  while start <= #source do
    local newline = string.find(source, '\n', start, true)
    if not newline then
      table.insert(result, string.sub(source, start))
      break
    end

    local finish = newline - 1
    if string.byte(source, finish) == 13 then finish -= 1 end

    table.insert(result, string.sub(source, start, finish))
    start = newline + 1
  end

  return result
end

function Oracle.parse(source: string, path: string): { Entry }
  local valid, invalid = utf8.len(source)
  if not valid then Oracle.fail(path, 0, `configuration is not UTF-8 at byte {invalid}`) end

  local entries: { Entry }, configs: { Entry }, comments: { string } = {}, {}, {}
  local encodingAt: number?

  for number, line in Oracle.lines(source) do
    local trimmed = text.trim(line)
    if trimmed == '' then
      table.insert(comments, '\n')
      continue
    end

    if string.sub(trimmed, 1, 1) == '#' then
      table.insert(comments, line .. '\n')
      continue
    end

    local equals = string.find(trimmed, '=', 1, true) or Oracle.fail(path, number, `invalid line '{trimmed}'`)

    local key = text.trim(string.sub(trimmed, 1, equals - 1))
    local value = text.trim(string.sub(trimmed, equals + 1))
    Oracle.validate(key, value, path, number)
    local entry: Entry = {
      key = key,
      value = value,
      source = path,
      line = number,
      comment = table.concat(comments),
    }
    table.clear(comments)
    if key == 'config' then
      table.insert(configs, entry)
    elseif key == 'encoding' and encodingAt then
      entries[encodingAt] = entry
    else
      table.insert(entries, entry)
      if key == 'encoding' then encodingAt = #entries end
    end
  end

  table.move(configs, 1, #configs, #entries + 1, entries)
  local trailing: { string } = {}

  for _, comment in comments do
    if string.sub(comment, 1, #Stamp) ~= Stamp then table.insert(trailing, comment) end
  end

  if #trailing > 0 then
    table.insert(entries, { source = path, line = 0, comment = table.concat(trailing) })
  end

  return entries
end

--- Every string of at most `longest` symbols of `alphabet`, the empty one first.
local function spellings(alphabet: { string }, longest: number): { string }
  local result, previous = { '' }, { '' }

  for _ = 1, longest do
    local grown = [for prefix in previous for symbol in alphabet => prefix .. symbol]

    for _, spelling in grown do
      table.insert(result, spelling)
    end

    previous = grown
  end

  return result
end

local function sameEntries(left: { Entry }, right: { Entry }): boolean
  if #left ~= #right then return false end

  for index, entry in left do
    local other = right[index]
    local same = entry.key == other.key and entry.value == other.value and entry.source == other.source
      and entry.line == other.line and entry.comment == other.comment and entry.resolved == other.resolved
    if not same then return false end
  end

  return true
end

--- What a call raised, without the position Luau puts before the text of a failed `assert`: the
--- oracle's lines are not the package's.
local function raised(problem: unknown): string
  return (string.gsub(tostring(problem), '^[^\n]-%.luau:%d+: ', ''))
end

--- Whether two calls did the same thing: returned equal values, or raised the same text.
local function agree(expectedPassed: boolean, expected: unknown, passed: boolean, actual: unknown): boolean
  if expectedPassed ~= passed then return false end

  if not passed then return raised(expected) == raised(actual) end

  if type(expected) == 'table' then return sameEntries(expected :: { Entry }, actual :: { Entry }) end

  return expected == actual
end

--- The first of `cases` that `shipped` and `oracle` treat differently, quoted, or `''`.
local function disagreement<Case>(
  cases: { Case },
  describe: (case: Case) -> string,
  oracle: (case: Case) -> unknown,
  shipped: (case: Case) -> unknown
): string
  for _, case in cases do
    -- Both are run to their error on purpose: a spelling the grammar refuses has to be refused
    -- by both, in the same words.
    local expectedPassed, expected = pcall(oracle, case)
    -- The shipped one raises on the same spellings, which is what is being compared.
    local passed, actual = pcall(shipped, case)
    if not agree(expectedPassed, expected, passed, actual) then
      return `{describe(case)}: expected {expectedPassed} {expected}, got {passed} {actual}`
    end
  end

  return ''
end

local function quoted(case: string): string return string.format('%q', case) end

local Source = jess.fs.join('chain', 'user', 'openmw.cfg')

suite.describe('path keys and resolution', function()
  suite.it('keys every short spelling of a path as the segment walk does', function()
    local cases = spellings({ '/', '\\', '.', 'a' }, 8)
    table.insert(cases, '/home/player/.config/openmw')
    table.insert(cases, '/home/player/.local/share/openmw/mods/A Mod/Data Files/')
    table.insert(cases, 'C:\\Games\\Morrowind\\.\\Data Files')
    table.insert(cases, '\\\\server\\share\\mods')
    table.insert(cases, 'relative/./folder//with spaces/..')
    expect(#cases).toBe(87386)
    expect(disagreement(cases, quoted, Oracle.key, paths.key)).toBe ''
    expect(disagreement(cases, quoted, Oracle.separators, paths.separators)).toBe ''
  end)

  suite.it('unquotes and resolves every short spelling of a value as the character walk does', function()
    local cases = spellings({ '"', '&', 'a', '/', '\\', '?' }, 6)

    for _, drive in { 'C:/a', 'C:\\a', 'C:a', '"C:/a"', 'c:', '1:/a' } do
      table.insert(cases, drive)
    end

    local function resolved(resolve: typeof(paths.resolve)): (case: string) -> unknown
      return function(case: string): unknown return resolve(case, Source, {}) end
    end

    expect(#cases).toBe(55993)
    expect(disagreement(cases, quoted, resolved(Oracle.resolve), resolved(paths.resolve))).toBe ''
  end)

  suite.it('resolves tokens, and refuses one with no root, as the token table walk does', function()
    local prefixes = { '?local?', '?global?', '?userconfig?', '?userdata?', '?other?', '?LOCAL?', '?local', 'a?local?', '' }
    local suffixes = { '', 'a', '/a', '\\a', '//a/./b/', '?local?', 'mods/&"A Mod&"', '"' }
    local roots: Tokens = {
      localPath = jess.fs.join('roots', 'local'),
      globalPath = jess.fs.join('roots', 'global'),
      userConfig = jess.fs.join('roots', 'config'),
      userData = jess.fs.join('roots', 'data'),
    }

    type Case = { value: string, tokens: Tokens }

    local cases: { Case } = {}

    for _, prefix in prefixes do
      for _, suffix in suffixes do
        for _, value in { prefix .. suffix, `"{prefix}{suffix}"`, `"{prefix}&{suffix}` } do
          table.insert(cases, { value = value, tokens = roots })
          table.insert(cases, { value = value, tokens = {} })
          table.insert(cases, { value = value, tokens = { userData = roots.userData } })
        end
      end
    end

    local function resolved(resolve: typeof(paths.resolve)): (case: Case) -> unknown
      return function(case: Case): unknown return resolve(case.value, Source, case.tokens) end
    end

    local function describe(case: Case): string return quoted(case.value) end

    expect(#cases).toBe(648)
    expect(disagreement(cases, describe, resolved(Oracle.resolve), resolved(paths.resolve))).toBe ''
  end)
end)

suite.describe('the grammar', function()
  suite.it('validates every short key and value as the pattern classes do', function()
    local keys = spellings({ 'a', '=', '\r', '\n', '\0', ' ', '#' }, 3)
    local values = spellings({ 'a', '\r', '\n', '\0', ' ', ',', '\u{A0}', '\xFF' }, 3)

    for _, key in { 'encoding', 'fallback', 'content', 'data', '\u{3000}data', 'da\xFFta' } do
      table.insert(keys, key)
    end

    for _, value in { 'win1250', 'win1251', 'win1252', 'utf8', 'Key,Value', 'Key', 'Morrowind.esm' } do
      table.insert(values, value)
    end

    type Case = { key: string, value: string }

    local cases: { Case } = [for key in keys for value in values => { key = key, value = value }]

    local function validated(validate: typeof(text.validate)): (case: Case) -> unknown
      return function(case: Case): unknown
        validate(case.key, case.value, Source, 7)
        return true
      end
    end

    local function describe(case: Case): string return `{quoted(case.key)} = {quoted(case.value)}` end

    expect(#cases).toBe(406 * 592)
    expect(disagreement(cases, describe, validated(Oracle.validate), validated(text.validate))).toBe ''
  end)

  suite.it('refuses a key or a value that is not a string, whatever else it is', function()
    local validate = (text.validate :: unknown) :: (key: unknown, value: unknown, source: string, line: number) -> ()
    expect(function() validate(nil, 'value', Source, 3) end).toThrow 'settings need string keys and values'
    expect(function() validate('content', 7, Source, 3) end).toThrow 'settings need string keys and values'
    expect(function() validate('content', {}, Source, 3) end).toThrow 'settings need string keys and values'
  end)

  suite.it('quotes every short directory for export as the pattern classes do', function()
    local cases = spellings({ 'a', ' ', '"', '&', '\r', '\n', '\0', '\u{A0}', '\xFF' }, 5)
    expect(#cases).toBe(66430)
    expect(disagreement(cases, quoted, Oracle.encodeDirectory, text.encodeDirectory)).toBe ''
  end)

  suite.it('reads every short file as a validation of every setting does', function()
    local fragments = {
      'content=Morrowind.esm',
      '\t data = "mods/A Mod" ',
      '# a comment',
      '',
      'content=Bro\rken.esp',
      'conte\0nt=Morrowind.esm',
      '=value',
      'encoding=win1251',
      'encoding=utf8',
      'fallback=Key',
      'config=user',
      'nonsense',
      '\u{A0}fallback\u{2003}=\u{3000}Key,Value\u{A0}',
      `{Stamp} 1`,
    }

    local endings = { '\n', '\r\n', '\r\r\n' }
    local lines = [for fragment in fragments for ending in endings => fragment .. ending]
    local cases = [for first in lines for second in lines for last in fragments => first .. second .. last]

    for _, first in lines do
      for _, second in lines do
        table.insert(cases, first .. second)
      end
    end

    for _, alone in { '', '\r', '\n', '\r\n', '# only\r', 'content=A.esp\r', 'content=A.esp\0\n', 'content=\xFFA.esp\n' } do
      table.insert(cases, alone)
    end

    local function parsed(parse: typeof(text.parse)): (case: string) -> unknown
      return function(case: string): unknown return parse(case, Source) end
    end

    expect(#cases).toBe(42 * 42 * 14 + 42 * 42 + 8)
    expect(disagreement(cases, quoted, parsed(Oracle.parse), parsed(text.parse))).toBe ''
  end)
end)

return suite
