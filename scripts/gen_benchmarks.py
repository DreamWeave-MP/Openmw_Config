#!/usr/bin/env python3
"""
Generate BENCHMARKS.md from Criterion benchmark output.

Run `cargo bench --bench parsing` first, then:
    python3 scripts/gen_benchmarks.py

Reads:  target/criterion/**/new/{benchmark,estimates}.json
Writes: BENCHMARKS.md
"""

import json
import math
import sys
from collections import defaultdict
from datetime import date
from pathlib import Path


CRITERION_DIR = Path("target/criterion")
OUTPUT_FILE = Path("BENCHMARKS.md")


# ---------------------------------------------------------------------------
# Data loading
# ---------------------------------------------------------------------------

def load_benchmarks() -> list[dict]:
    records = []
    for estimates_path in sorted(CRITERION_DIR.rglob("new/estimates.json")):
        bench_path = estimates_path.parent / "benchmark.json"
        if not bench_path.exists():
            continue
        meta = json.loads(bench_path.read_text())
        ests = json.loads(estimates_path.read_text())
        records.append({
            "group_id":   meta["group_id"],
            "function_id": meta.get("function_id"),
            "value_str":  meta.get("value_str"),
            "mean_ns":    ests["mean"]["point_estimate"],
            "std_ns":     ests["std_dev"]["point_estimate"],
        })
    return records


# ---------------------------------------------------------------------------
# Time helpers
# ---------------------------------------------------------------------------

def best_unit(values_ns: list[float]) -> tuple[float, str]:
    """Pick ns / µs / ms based on the largest value in the list."""
    m = max(values_ns)
    if m < 1_000:
        return 1.0, "ns"
    elif m < 1_000_000:
        return 1_000.0, "µs"
    else:
        return 1_000_000.0, "ms"


def fmt_time(ns: float, divisor: float, unit: str) -> str:
    v = ns / divisor
    return f"{v:.1f} {unit}" if v >= 100 else f"{v:.2f} {unit}"


def nice_ceil(v: float) -> float:
    """Round v up to the next 'nice' number (1/2/5 × 10^n)."""
    if v <= 0:
        return 1.0
    mag = 10 ** math.floor(math.log10(v))
    for step in (1, 2, 5, 10):
        candidate = step * mag
        if candidate >= v:
            return candidate
    return 10 * mag  # unreachable, satisfies type checker


# ---------------------------------------------------------------------------
# Mermaid chart generation
# ---------------------------------------------------------------------------

def safe_mermaid_title(s: str) -> str:
    """Escape double-quotes in mermaid chart titles."""
    return s.replace('"', "'")


def chart_label(variant: str) -> str:
    """
    Shorten a variant name for an x-axis label.
    'small (10 dirs, 10 plugins, 50 fallbacks)' → 'small'
    'relative' → 'relative'
    """
    paren = variant.find("(")
    return variant[:paren].strip() if paren != -1 else variant


def make_bar_chart(title: str, labels: list[str], values_ns: list[float]) -> str:
    """Single-series xychart-beta bar chart."""
    div, unit = best_unit(values_ns)
    scaled = [v / div for v in values_ns]
    y_max = nice_ceil(max(scaled) * 1.2)
    x_axis = ", ".join(f'"{l}"' for l in labels)
    data = ", ".join(f"{v:.2f}" for v in scaled)
    return "\n".join([
        "```mermaid",
        "xychart-beta",
        f'    title "{safe_mermaid_title(title)}"',
        f"    x-axis [{x_axis}]",
        f'    y-axis "time ({unit})" 0 --> {y_max:.2f}',
        f"    bar [{data}]",
        "```",
    ])


# ---------------------------------------------------------------------------
# Benchmark organisation
# ---------------------------------------------------------------------------

def standalone_prefix(group_id: str) -> str:
    """
    'DirectorySetting::new relative'           → 'DirectorySetting::new'
    'DirectorySetting::new ?userdata? token'   → 'DirectorySetting::new'
    'OpenMWConfiguration::new small (10 dirs…)' → 'OpenMWConfiguration::new'
    """
    return group_id.split()[0]


def try_int(s: str):
    try:
        return int(s)
    except ValueError:
        return s


# ---------------------------------------------------------------------------
# Markdown generation
# ---------------------------------------------------------------------------

def generate_md(records: list[dict]) -> str:
    lines: list[str] = [
        "# Benchmarks",
        "",
        (
            f"> Generated {date.today()} · "
            "`cargo bench --bench parsing` → Criterion → "
            "[scripts/gen_benchmarks.py](scripts/gen_benchmarks.py)"
        ),
        "",
        "All times are wall-clock means measured by "
        "[Criterion.rs](https://github.com/bheisler/criterion.rs) "
        "(95 % confidence interval).",
        "",
    ]

    # Split into standalone bench_function calls vs criterion_group benchmarks
    param_groups: dict[str, list[dict]] = defaultdict(list)  # group_id → records
    standalone_map: dict[str, list[dict]] = defaultdict(list)  # prefix  → records

    for r in records:
        if r["function_id"] is not None or r["value_str"] is not None:
            param_groups[r["group_id"]].append(r)
        else:
            standalone_map[standalone_prefix(r["group_id"])].append(r)

    # ------------------------------------------------------------------
    # Standalone groups (e.g. DirectorySetting::new, OpenMWConfiguration::new)
    # ------------------------------------------------------------------
    for prefix, recs in sorted(standalone_map.items()):
        recs.sort(key=lambda r: r["group_id"])
        lines.append(f"## {prefix}")
        lines.append("")

        all_ns = [r["mean_ns"] for r in recs]
        div, unit = best_unit(all_ns)

        lines.append("| Variant | Mean | ± Std Dev |")
        lines.append("|---|---:|---:|")
        for r in recs:
            variant = r["group_id"][len(prefix):].strip() or r["group_id"]
            lines.append(
                f"| {variant} "
                f"| {fmt_time(r['mean_ns'], div, unit)} "
                f"| {fmt_time(r['std_ns'], div, unit)} |"
            )
        lines.append("")

        chart_labels = []
        for r in recs:
            variant = r["group_id"][len(prefix):].strip() or r["group_id"]
            chart_labels.append(chart_label(variant))

        lines.append(make_bar_chart(prefix, chart_labels, all_ns))
        lines.append("")

    # ------------------------------------------------------------------
    # Parametric groups (e.g. has_content_file, get_game_setting, …)
    # ------------------------------------------------------------------
    for group_id, recs in sorted(param_groups.items()):
        lines.append(f"## {group_id}")
        lines.append("")

        # fn_id → value_str → record
        fn_map: dict[str, dict[str, dict]] = defaultdict(dict)
        for r in recs:
            fn_map[r["function_id"] or ""][r["value_str"] or ""] = r

        # x-axis: input sizes sorted numerically
        all_x = sorted(
            {r["value_str"] for r in recs if r["value_str"]},
            key=try_int,
        )
        fn_ids = sorted(fn_map.keys())

        all_ns = [r["mean_ns"] for r in recs]
        div, unit = best_unit(all_ns)

        # Summary table
        header = "| n | " + " | ".join(fn_ids) + " |"
        sep    = "|---|" + "---:|" * len(fn_ids)
        lines.extend([header, sep])
        for x in all_x:
            cells = [
                fmt_time(fn_map[fn_id][x]["mean_ns"], div, unit)
                if x in fn_map[fn_id] else "—"
                for fn_id in fn_ids
            ]
            lines.append(f"| {x} | " + " | ".join(cells) + " |")
        lines.append("")

        # One chart per function_id
        for fn_id in fn_ids:
            fn_ns = [
                fn_map[fn_id][x]["mean_ns"] if x in fn_map[fn_id] else 0.0
                for x in all_x
            ]
            chart_title = f"{group_id} / {fn_id}" if fn_id else group_id
            lines.append(f"**{fn_id or group_id}**")
            lines.append("")
            lines.append(make_bar_chart(chart_title, all_x, fn_ns))
            lines.append("")

    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    if not CRITERION_DIR.exists():
        print(
            "error: target/criterion not found — run `cargo bench --bench parsing` first",
            file=sys.stderr,
        )
        sys.exit(1)

    records = load_benchmarks()
    if not records:
        print(
            "error: no benchmark results found under target/criterion",
            file=sys.stderr,
        )
        sys.exit(1)

    md = generate_md(records)
    OUTPUT_FILE.write_text(md, encoding="utf-8")
    print(f"wrote {OUTPUT_FILE}  ({len(records)} benchmark result(s))")


if __name__ == "__main__":
    main()
