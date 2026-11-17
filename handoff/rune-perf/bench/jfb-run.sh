#!/bin/bash
# Interleaved jfb runs: each round visits every op, and within an op every
# runtime in a fresh random order, each a fresh process taking 3 samples.
# Usage: jfb-run.sh ROUNDS OUT.jsonl
# Point these at your own builds (see HANDOFF.md, "How to run the benches").
RS=${RS:?set RS to the rust-solid jfb example binary}
RU=${RU:?set RU to the rune target/perf/examples directory}
EX=${EX:?set EX to the exact-runner jfb example binary}
declare -A CMD=(
  [rust-solid]="$RS"
  [rune]="$RU/jfb_rune"
  [rune-memo]="env RUNE_MEMO=1 $RU/jfb_rune"
  [rune-virtual]="env RUNE_VIRTUAL=1 $RU/jfb_rune"
  [glue-retained]="$RU/jfb_glue"
  [glue-retained-lazy]="env GLUE_LAZY=1 $RU/jfb_glue"
  [exact]="$EX"
  [exact-layout]="env EXACT_LAYOUT=1 $EX"
)
OPS="run replace update select swap remove runlots add clear"
for round in $(seq 1 $1); do
  for op in $OPS; do
    for rt in $(printf '%s\n' "${!CMD[@]}" | shuf); do
      ${CMD[$rt]} $op 3 >> $2 2>> $2.err || echo "FAIL $rt $op round $round" >> $2.err
    done
  done
  echo "round $round done $(date +%T)" >> $2.err
done
