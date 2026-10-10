"""Summarises PLAY_LOG_PERF CSVs (crates/game/src/perf_log.rs): gameplay frame times and every
slow frame with what happened in it.

Usage: uv run tools/perf/summary.py [-v] [--fail-over MS] cache/perf/<tag>-*.csv

Gameplay frames run from the second frame after the first simulation step to the frame before
the replay's exit sleep. With --fail-over, exits 1 when a gameplay frame without a hit is slower
than MS milliseconds (the warm-run check in STATUS.md, "Frame pacing").
"""

import argparse
import csv
import statistics
import sys


def percentile(values, p):
    s = sorted(values)
    k = (len(s) - 1) * p / 100
    lo = int(k)
    hi = min(lo + 1, len(s) - 1)
    return s[lo] + (s[hi] - s[lo]) * (k - lo)


def events(row, prev, seen, extra):
    out = []
    for side, key in ((0, "wolf_clip"), (1, "owl_clip")):
        if row[key] not in seen[side]:
            out.append(f"first {row[key]}")
            seen[side].add(row[key])
    for who, key in (("wolf", "wolf_state"), ("owl", "owl_state")):
        if row[key] != prev[key]:
            out.append(f"{who} {row[key].split('/')[-1]}")
    if int(row["wolf_starts"]) or int(row["owl_starts"]):
        out.append(f"clip starts {row['wolf_starts']}/{row['owl_starts']}")
    if int(row["hits"]):
        out.append(f"hits {row['hits']}")
    if int(row["sounds"]):
        out.append(f"sounds {row['sounds']}")
    for k in extra:
        if row[k] not in ("", "0", "0.000", "-"):
            out.append(f"{k[2:]}={row[k]}")
    return out


def analyse(path, threshold, verbose, fail_over):
    rows = list(csv.DictReader(open(path, encoding="utf-8")))
    start = next(i for i, r in enumerate(rows) if int(r["sim_step"]) > 0)
    game = rows[start + 1 : -1]
    ms = [float(r["wall_ms"]) for r in game]
    seen = [{r["wolf_clip"] for r in rows[: start + 1]}, {r["owl_clip"] for r in rows[: start + 1]}]
    extra = [k for k in rows[0] if k.startswith("x_")]
    slow, failures = [], []
    prev = rows[start]
    for r in game:
        what = events(r, prev, seen, extra)
        wall = float(r["wall_ms"])
        if wall > threshold:
            slow.append((r["frame"], wall, r["steps"], ", ".join(what) or "-"))
        if fail_over is not None and wall > fail_over and not int(r["hits"]):
            failures.append(r["frame"])
        prev = r
    steps = [int(r["steps"]) for r in game]
    print(
        f"{path}: {len(game)} frames, median {statistics.median(ms):.2f} ms, "
        f"p95 {percentile(ms, 95):.2f}, p99 {percentile(ms, 99):.2f}, max {max(ms):.2f}, "
        f"{len(slow)} over {threshold:g} ms, steps per frame 0/1/2/3+ "
        f"{steps.count(0)}/{steps.count(1)}/{steps.count(2)}/{sum(s >= 3 for s in steps)}, "
        f"hits {sum(int(r['hits']) for r in game)}"
    )
    if verbose:
        for frame, wall, n, what in slow:
            print(f"    frame {frame}: {wall:.1f} ms, {n} steps, {what}")
    if failures:
        print(f"    FAIL: frames over {fail_over:g} ms without a hit: {', '.join(failures)}")
    return not failures


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", nargs="+")
    parser.add_argument("-v", action="store_true", help="list every slow frame")
    parser.add_argument("--threshold", type=float, default=25.0)
    parser.add_argument("--fail-over", type=float)
    args = parser.parse_args()
    ok = all([analyse(p, args.threshold, args.v, args.fail_over) for p in args.csv])
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
