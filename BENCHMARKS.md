# Configuration benchmarks

`bench/bench.luau` times the package through its public API: the grammar (reading a file,
validating, trimming, quoting, rendering), path keys and resolution, loading a chain of three
files the size of a large mod list's, its queries and edits, and rendering and saving the user
file. Each workload is warmed up, sized to a batch of about a tenth of a second, and reported as
the median of seven batches, with a full collection between them. Every workload's result is
checked against what the generator of its input says it must be.

`bench/cold.luau` times what one process pays once: the `require` of each of the three modules,
and the first load of a small chain. Each line is a single sample, so it is run many times and the
medians are compared.

```sh
jess run --package jpk/openmw-config --package-path jpk \
  --capability filesystem.read --capability filesystem.write \
  jpk/openmw-config/bench/bench.luau <scratch folder> [filter]

jess run --package jpk/openmw-config --package-path jpk \
  --capability filesystem.read --capability filesystem.write \
  jpk/openmw-config/bench/cold.luau <scratch folder>
```

## The move onto l3i 2.0, base against head

- Machine: Intel Core i7-10870H @ 2.20 GHz, 16 threads, shared with other work while this ran.
- Date: 2026-10-12.
- Binary: the verified release build of Jess 0.1.0 on l3i 2.0.2 (Luau 0.742), the same one for
  both sides.
- Base: `fb6bfba`, the package before the move, with both benches. Head: the source of `2fd2037`,
  which no later commit changes.
- Protocol: the whole of `bench.luau` against base, then head, five times over, back to back
  under one lock; each cell is the median of its five runs. `cold.luau` the same way, forty-one
  times over. A difference under 5 percent is called no change.

| Workload | Base, µs | Head, µs | Base / head |
|---|---:|---:|---:|
| text.parse: a mod list's openmw.cfg (1325 lines) | 2460.0 | 792.4 | 3.10x |
| text.parse: the same written by hand, CRLF (1325 lines) | 2726.0 | 1102.0 | 2.47x |
| text.validate: 1286 settings | 1522.0 | 660.7 | 2.30x |
| text.render: 1286 entries | 201.0 | 198.1 | no change |
| text.trim: 1024 values, nothing to trim | 50.4 | 49.8 | no change |
| text.trim: 1024 values, spaces and tabs | 153.3 | 152.9 | no change |
| text.trim: 1024 values, Unicode spaces | 585.4 | 566.9 | no change |
| paths.resolve: 1024 data folders | 4004.0 | 543.6 | 7.37x |
| paths.key: 1024 folders | 3105.0 | 547.8 | 5.67x |
| text.encodeDirectory: 1024 folders | 1843.0 | 621.6 | 2.96x |
| new: a chain of three files (1393 lines) | 4742.0 | 1437.0 | 3.30x |
| values: content, data and fallback of 1351 settings | 61.6 | 60.3 | no change |
| dataDirectories: 423 folders | 46.1 | 46.8 | no change |
| entries: 1351 settings | 162.4 | 130.2 | 1.25x |
| render: 1351 settings | 299.0 | 304.3 | no change |
| render, resolved: 1351 settings | 967.6 | 482.1 | 2.01x |
| remove and append: one of 421 data folders | 2431.0 | 162.0 | 15.00x |
| renderUser: 400 inherited data folders relisted | 4376.0 | 696.4 | 6.28x |
| replace: 300 content files of 340 | 337.3 | 244.5 | 1.38x |
| set and path: resources | 2.74 | 1.98 | 1.38x |
| saveUser: a user file of 60 settings | 1011.0 | 995.2 | no change |

| One cold process | Base, µs | Head, µs | Base / head |
|---|---:|---:|---:|
| require @openmw-config/paths | 935.1 | 1334.0 | 0.70x |
| require @openmw-config/text | 2016.0 | 2428.0 | 0.83x |
| require @openmw-config, the two above loaded | 6583.0 | 6991.0 | 0.94x |
| require all three | 9575.0 | 10724.0 | 0.89x |
| first new, a chain of two files (127 lines) | 661.4 | 271.6 | 2.44x |

### What moved, and why

None of the warm gain is the dialect's, bar one row. The package spent its time in Lua pattern
classes and in walks a character at a time, and those are what went. A setting's forbidden bytes
(`[\r\n%z]`, 740 ns over a 50-byte value) are found by three plain searches (110 ns), which is
`text.validate` and `text.encodeDirectory`. A reader that has checked its file once for NUL and
for a carriage return that ends no line (`bytes.count`, twice) and for UTF-8 (`bytes.isUtf8`)
has nothing left to refuse in a key and a value it cut and trimmed itself but an empty key, a bad
encoding and a fallback with no comma, which with the plain searches is `text.parse`. A path's
other separator is translated by `bytes.translate`; a quoted value with no `&` in it is cut out
whole; the token table is built once and consulted only for a path that opens with `?`: together,
`paths.resolve`. A path that is already its own key is returned as it is, and one that is not is
split at its separators instead of walked by a pattern iterator: `paths.key`. `remove` takes the
key of the folder it was given once instead of once per entry. The chain rows (`new`, `render,
resolved`, `remove and append`, `renderUser`, `replace`, `set and path`) are those gains reaching
the callers.

The dialect's own row is `entries` (1.25x: a comprehension over `table.clone` instead of
`table.insert` in a generic `for`), and 1.09x of `paths.key` measured on its own commit, from the
comprehension that filters a split path's segments. The other five constructs in the payload (two
more comprehensions, two `any`, a record pattern) are in code that runs once per file or per call
and move nothing.

The cold process is the other side, and it is slower to load: 1.15 ms more to `require` the three
modules. About half is the code the fast paths added (some 70 lines; a module here costs roughly
10 µs a line to load, with or without `--!native`). The rest is the dialect. With every dialect
construct in the payload put back to its loop and nothing else changed, forty-one runs each, twice:

| require all three | No dialect, µs | Head, µs | |
|---|---:|---:|---:|
| first run of 41 | 10238.0 | 10924.0 | 0.94x |
| second run of 41 | 10042.0 | 10473.0 | 0.96x |

So seven constructs cost 0.4 to 0.7 ms of load, against the 32 µs a call `entries` saves on a
chain of 1,351 settings. For a command that loads a 127-line chain once and exits, base is
9.58 + 0.66 = 10.24 ms and head is 10.72 + 0.27 = 11.00 ms: 7 percent slower. Summing the measured
rows for a chain the size of a mod list's (`require all three` and the warm `new`), base is
14.3 ms and head 12.2 ms. The crossover is a chain of several hundred lines. Each dialect
conversion is its own commit and reverts cleanly if load time is what counts.

Measured and not kept: reading lines with `string.split` (no change on LF, 0.94x on CRLF);
`bytes.isUtf8` for a key and a value in `text.validate` (1.06x on its row, nothing on `replace`:
noise); `values` as a comprehension, which needs a filter on the value so that a key that is no
string still matches nothing (0.87x).
