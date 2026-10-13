--!strict
-- What a command that loads one configuration and exits pays before anything is warm: the
-- package's three modules compiled and run, then the first load of a small chain. Run by `jess run
-- --capability filesystem.read --capability filesystem.write --package . bench/cold.luau
-- <scratch>`, on a release build. Each line is one sample of something a process does once, so
-- compare medians over many runs. This file uses none of the l3i dialect, so that whatever a
-- module's first use of it costs is paid inside the `require` timed here.
local fs = require '@dream/fs'

if not jess.bench.verified then
  error('config bench: this jess was built without the verified toolchain: measure on a release build', 0)
end

local scratch: string? = ...
local root = jess.fs.join(assert(scratch, 'config bench: the first argument is a scratch folder'), 'openmw-config-cold')
local userData = jess.fs.join(root, 'userdata')
local installPath = jess.fs.join(root, 'install', 'openmw.cfg')
local userPath = jess.fs.join(root, 'user', 'openmw.cfg')

local function io<Value>(value: Value?, message: string?, kind: dream_fs_ErrorKind?): Value
  if not value then error(message or tostring(kind), 0) end

  return value
end

local function write(path: string, lines: { string })
  io(fs.mkdir(jess.fs.parent(path) :: string, { recursive = true }))
  io(fs.writeFile(path, table.concat(lines, '\n') .. '\n'))
end

local Fallbacks, Folders, Plugins = 60, 30, 30
local user = { '# Written by the launcher.', 'encoding=win1252' }

for index = 1, Fallbacks do
  table.insert(user, `fallback=Weather_Clear_Sky_Day_Color_{index},{index % 255},{index * 7 % 255},{index * 13 % 255}`)
end

for index = 1, Folders do
  table.insert(user, `data="/home/player/.local/share/openmw/mods/Landscape Retexture {index}/Data Files"`)
end

for index = 1, Plugins do
  table.insert(user, `content=Landscape Retexture {index}.esp`)
end

local install = {
  '# The engine\'s own settings.',
  'data-local="?userdata?data"',
  'resources=./resources',
  'data=./resources/vfs-mw',
  `config="{jess.fs.parent(userPath) :: string}"`,
}

write(installPath, install)
write(userPath, user)

local started = os.clock()
local configuration = require '@openmw-config'
local required = os.clock() - started

started = os.clock()
local config = configuration.new(installPath, { tokens = { userData = userData } })
local loaded = os.clock() - started

io(fs.remove(root, { recursive = true }))

if #config:values('content') ~= Plugins or #config:dataDirectories() ~= Folders + 3 or #config:chain() ~= 2 then
  error('config bench: the cold load is not what the files hold', 0)
end

print(`cold: require the package\t{string.format('%.9f', required)}`)
print(`cold: first new, a chain of two files ({#install + #user} lines)\t{string.format('%.9f', loaded)}`)
