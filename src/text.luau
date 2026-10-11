--!strict
--!native
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Configuration grammar follows openmw-config 3.0.2's ConfigFile::read. Fallback values stay
-- strings: their application meaning and i64 precision do not belong to the configuration loader.

export type Entry = {
  key: string?,
  value: string?,
  source: string,
  line: number,
  comment: string,
  resolved: string?,
}

local DirectoryKeys: { [string]: boolean } = {
  data = true,
  config = true,
  resources = true,
  ['data-local'] = true,
  ['user-data'] = true,
}

local SingletonKeys: { [string]: boolean } = {
  encoding = true,
  resources = true,
  ['data-local'] = true,
  ['user-data'] = true,
}

local UniqueKeys: { [string]: boolean } = { content = true, groundcover = true, ['fallback-archive'] = true }
table.freeze(DirectoryKeys)
table.freeze(SingletonKeys)
table.freeze(UniqueKeys)
local Stamp = '# OpenMW-Config Serializer Version:'

local function whitespace(code: number): boolean
  return code == 32 or (code >= 9 and code <= 13) or code == 133 or code == 160 or code == 5760
    or (code >= 8192 and code <= 8202) or code == 8232 or code == 8233 or code == 8239
    or code == 8287 or code == 12288
end

local function asciiSpace(code: number): boolean return code == 32 or (code >= 9 and code <= 13) end

-- Shared with the INI importer: Rust's Unicode whitespace trimming, with an ASCII fast path.
local function trim(value: string): string
  local first, last = 1, #value
  while first <= last and asciiSpace(string.byte(value, first)) do first += 1 end

  while last >= first and asciiSpace(string.byte(value, last)) do last -= 1 end

  if first > last then return '' end

  local lead, tail = string.byte(value, first), string.byte(value, last)
  if lead ~= 194 and lead ~= 225 and lead ~= 226 and lead ~= 227 and tail < 128 then
    return if first == 1 and last == #value then value else string.sub(value, first, last)
  end

  value = string.sub(value, first, last)
  first, last = 1, #value
  for index, code in utf8.codes(value) do
    if not whitespace(code) then
      first = index
      break
    end

    first = index + #utf8.char(code)
  end

  if first > last then return '' end

  while last >= first do
    local index = utf8.offset(value, -1, last + 1) :: number
    if not whitespace(utf8.codepoint(value, index)) then break end

    last = index - 1
  end

  return string.sub(value, first, last)
end

local function fail(source: string, line: number, reason: string): never
  return error(`dreamwind.openmw-config: {source}:{line}: {reason}`, 0)
end

local function validate(key: string, value: string, source: string, line: number)
  if type(key) ~= 'string' or type(value) ~= 'string' then
    fail(source, line, 'settings need string keys and values')
  end

  if not utf8.len(key) or not utf8.len(value) then fail(source, line, 'settings must be UTF-8') end

  if key == '' or string.sub(key, 1, 1) == '#' or string.find(key, '[=\r\n%z]') or string.find(value, '[\r\n%z]') then
    fail(source, line, 'a setting must fit on one key=value line without NUL bytes')
  end

  if trim(key) ~= key or trim(value) ~= value then
    fail(source, line, 'setting keys and values must not have surrounding whitespace')
  end

  if key == 'encoding' and value ~= 'win1250' and value ~= 'win1251' and value ~= 'win1252' then
    fail(source, line, `unsupported encoding '{value}'`)
  end

  if key == 'fallback' and not string.find(value, ',', 1, true) then
    fail(source, line, `fallback needs Key,Value, got '{value}'`)
  end
end

local function encodeDirectory(value: string): string
  assert(utf8.len(value) and not string.find(value, '[\r\n%z]'),
    'dreamwind.openmw-config: directory paths must be single-line UTF-8 for export')
  if trim(value) == value and string.sub(value, 1, 1) ~= '"' then return value end

  return '"' .. string.gsub(value, '[&"]', function(character: string): string
    return '&' .. character
  end) .. '"'
end

local function lines(source: string): { string }
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

local function parse(source: string, path: string): { Entry }
  local valid, invalid = utf8.len(source)
  if not valid then fail(path, 0, `configuration is not UTF-8 at byte {invalid}`) end

  local entries: { Entry }, configs: { Entry }, comments: { string } = {}, {}, {}
  local encodingAt: number?
  for number, line in lines(source) do
    local trimmed = trim(line)
    if trimmed == '' then
      table.insert(comments, '\n')
      continue
    end

    if string.sub(trimmed, 1, 1) == '#' then
      table.insert(comments, line .. '\n')
      continue
    end

    local equals = string.find(trimmed, '=', 1, true) or fail(path, number, `invalid line '{trimmed}'`)

    local key = trim(string.sub(trimmed, 1, equals - 1))
    local value = trim(string.sub(trimmed, equals + 1))
    validate(key, value, path, number)
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

local function render(entry: Entry, resolved: boolean?): string
  if not entry.key then return entry.comment end

  local value = if resolved and entry.resolved then encodeDirectory(entry.resolved) else entry.value
  return `{entry.comment}{entry.key}={value}\n`
end

return table.freeze {
  DirectoryKeys = DirectoryKeys,
  SingletonKeys = SingletonKeys,
  UniqueKeys = UniqueKeys,
  fail = fail,
  validate = validate,
  parse = parse,
  render = render,
  trim = trim,
  encodeDirectory = encodeDirectory,
}
