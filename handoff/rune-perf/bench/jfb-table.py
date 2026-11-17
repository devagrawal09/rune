#!/usr/bin/env python3
"""Median and spread per runtime and op from jfb-run.sh's JSONL, as a Markdown table (ms)."""
import json, statistics, sys

ORDER = ["rust-solid", "exact", "exact-layout", "rune", "rune-memo", "rune-virtual", "glue-retained", "glue-retained-lazy"]
OPS = ["run", "replace", "update", "select", "swap", "remove", "runlots", "add", "clear"]
s = {}
for line in open(sys.argv[1]):
    if line.startswith("{"):
        r = json.loads(line)
        s.setdefault((r["runtime"], r["op"]), []).extend(r["us"])

def cell(v):
    if not v: return "–"
    m = statistics.median(v) / 1000
    return f"{m:,.2f}" if m < 10 else f"{m:,.1f}" if m < 100 else f"{m:,.0f}"

print("| Runtime | " + " | ".join(OPS) + " | n |")
print("|---|" + "---:|" * len(OPS) + "---:|")
for rt in ORDER:
    ns = {len(s.get((rt, op), [])) for op in OPS}
    print(f"| {rt} | " + " | ".join(cell(s.get((rt, op))) for op in OPS) + f" | {min(ns)} |")
if "--spread" in sys.argv:
    print("\nInterquartile range as % of the median:")
    for rt in ORDER:
        out = []
        for op in OPS:
            v = sorted(s.get((rt, op), []))
            if len(v) >= 4:
                q = statistics.quantiles(v, n=4)
                out.append(f"{op} {100 * (q[2] - q[0]) / statistics.median(v):.0f}%")
        print(f"- {rt}: " + ", ".join(out))
