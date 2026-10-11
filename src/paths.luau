--!strict
--!native
-- SPDX-License-Identifier: MIT OR Apache-2.0
local fs = require '@dream/fs'

export type Tokens = { localPath: string?, globalPath: string?, userConfig: string?, userData: string? }

local Windows = string.find(jess.fs.join('first', 'second'), '\\', 1, true) ~= nil
local Separator = if Windows then '\\' else '/'

local function separators(path: string): string return (string.gsub(path, '[/\\]', Separator)) end

local function rooted(path: string): boolean
  return string.sub(path, 1, 1) == Separator
    or (Windows and string.match(path, '^%a:[/\\]') ~= nil)
end

local function absolute(path: string): string
  if rooted(path) then return path end

  local result, message = fs.absolute(path)
  return assert(result, message)
end

-- Path equality is lexical, not canonical: symlink aliases remain distinct chain directories.
local function key(path: string): string
  local normalized = separators(path)
  local parts: { string } = {}
  for part in string.gmatch(normalized, '[^/\\]+') do
    if part ~= '.' then table.insert(parts, part) end
  end

  local prefix = if Windows and string.sub(normalized, 1, 2) == '\\\\'
    then '\\\\'
    elseif string.sub(normalized, 1, 1) == Separator then Separator else ''
  return prefix .. table.concat(parts, Separator)
end

local function unquote(value: string): string
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

local function resolve(value: string, source: string, tokens: Tokens): string
  local path = unquote(value)
  local roots: { [string]: string } = {
    ['?local?'] = 'localPath',
    ['?global?'] = 'globalPath',
    ['?userconfig?'] = 'userConfig',
    ['?userdata?'] = 'userData',
  }

  for token, field in roots do
    if string.sub(path, 1, #token) ~= token then continue end

    local root = ((tokens :: unknown) :: { [string]: string? })[field]
    assert(root, `dreamwind.openmw-config: {source}: token '{token}' needs an explicit '{field}' root`)
    local suffix = string.gsub(string.sub(path, #token + 1), '^[/\\]+', '')
    path = jess.fs.join(root, suffix)
    break
  end

  path = separators(path)
  if rooted(path) then return path end

  return jess.fs.join(jess.fs.parent(source) or '.', path)
end

return table.freeze { absolute = absolute, key = key, resolve = resolve, separators = separators }
