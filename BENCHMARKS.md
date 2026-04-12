# Benchmarks

> Generated 2026-04-12 · `cargo bench --bench parsing` → Criterion → [scripts/gen_benchmarks.py](scripts/gen_benchmarks.py)

All times are wall-clock means measured by [Criterion.rs](https://github.com/bheisler/criterion.rs) (95 % confidence interval).

## DirectorySetting::new

| Variant | Mean | ± Std Dev |
|---|---:|---:|
| ?userdata? token | 684.0 ns | 11.86 ns |
| absolute | 180.7 ns | 3.44 ns |
| quoted | 384.6 ns | 3.17 ns |
| relative | 159.1 ns | 2.21 ns |

```mermaid
xychart-beta
    title "DirectorySetting::new"
    x-axis ["?userdata? token", "absolute", "quoted", "relative"]
    y-axis "time (ns)" 0 --> 1000.00
    bar [684.04, 180.72, 384.58, 159.12]
```

## OpenMWConfiguration::new

| Variant | Mean | ± Std Dev |
|---|---:|---:|
| large (200 dirs, 500 plugins, 2000 fallbacks) | 6.30 ms | 0.05 ms |
| medium (50 dirs, 100 plugins, 500 fallbacks) | 1.53 ms | 0.07 ms |
| small (10 dirs, 10 plugins, 50 fallbacks) | 0.18 ms | 0.00 ms |

```mermaid
xychart-beta
    title "OpenMWConfiguration::new"
    x-axis ["large", "medium", "small"]
    y-axis "time (ms)" 0 --> 10.00
    bar [6.30, 1.53, 0.18]
```

## content_files_iter

| n | collect |
|---|---:|
| 10 | 3.75 ns |
| 100 | 33.63 ns |
| 500 | 246.4 ns |

**collect**

```mermaid
xychart-beta
    title "content_files_iter / collect"
    x-axis ["10", "100", "500"]
    y-axis "time (ns)" 0 --> 500.00
    bar [3.75, 33.63, 246.45]
```

## game_settings dedup

| n | unique_keys |
|---|---:|
| 50 | 6.15 µs |
| 250 | 38.87 µs |
| 1000 | 153.0 µs |

**unique_keys**

```mermaid
xychart-beta
    title "game_settings dedup / unique_keys"
    x-axis ["50", "250", "1000"]
    y-axis "time (µs)" 0 --> 200.00
    bar [6.15, 38.87, 153.01]
```

## get_game_setting

| n | found_last | not_found |
|---|---:|---:|
| 50 | 0.05 µs | 0.05 µs |
| 500 | 0.05 µs | 0.62 µs |
| 2000 | 0.05 µs | 2.39 µs |

**found_last**

```mermaid
xychart-beta
    title "get_game_setting / found_last"
    x-axis ["50", "500", "2000"]
    y-axis "time (ns)" 0 --> 100.00
    bar [47.87, 47.22, 48.24]
```

**not_found**

```mermaid
xychart-beta
    title "get_game_setting / not_found"
    x-axis ["50", "500", "2000"]
    y-axis "time (µs)" 0 --> 5.00
    bar [0.05, 0.62, 2.39]
```

## has_content_file

| n | found | not_found |
|---|---:|---:|
| 10 | 0.10 µs | 0.01 µs |
| 100 | 0.48 µs | 0.07 µs |
| 500 | 2.12 µs | 0.46 µs |

**found**

```mermaid
xychart-beta
    title "has_content_file / found"
    x-axis ["10", "100", "500"]
    y-axis "time (µs)" 0 --> 5.00
    bar [0.10, 0.48, 2.12]
```

**not_found**

```mermaid
xychart-beta
    title "has_content_file / not_found"
    x-axis ["10", "100", "500"]
    y-axis "time (ns)" 0 --> 1000.00
    bar [7.49, 69.59, 459.21]
```

