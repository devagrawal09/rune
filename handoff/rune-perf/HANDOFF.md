# Handoff: Rune as a renderer for rust-solid, and the performance work around it

This branch (`handoff/rune-perf` on devagrawal09/rune) holds two code commits on top of `dev` at
`29aacb68` and this folder. The folder is reference material for whoever picks the work up. It is
not meant to merge into Rune: before any upstream proposal, drop the commit that adds
`handoff/` and keep the two code commits.

## The goal

The wider project is a Rust port of Solid 2's runtime (`rust-solid`, on branch `solid-model` of
devagrawal09/reactivescript). It has no layout engine or window yet, so this work asked three
questions:

1. How does expo/exact, which compiles a UI language to a host-neutral plan and runs it on Rust
   hosts, compare with ReactiveScript and the `solid-model` Rust work? (REPORT.md)
2. How fast is rust-solid against Exact, and where does its time go? (PERF.md)
3. Does anomalyco/rune fit as rust-solid's renderer, and what would it need from Rune to make a
   fine-grained host cheap? (RUNE.md, the glue sections of PERF.md, RUNE-ISSUE-DRAFT.md)

## What's in this folder

| File | What it is |
|---|---|
| `REPORT.md` | Exact vs ReactiveScript `main` vs `solid-model`: architecture, overlap, who is ahead where, ideas to borrow. Read from the repos' own files, not from runs. |
| `PERF.md` | Every benchmark round, in order: Exact vs rust-solid micro-workloads, Rune, the glue, rust-solid profiling (rounds 1 and 2), and the headless js-framework-benchmark port. |
| `RUNE.md` | What Rune is, its reactivity model, where it sits beside Exact and solid-model, what to borrow, risks, and the glue prototype. |
| `RUNE-ISSUE-DRAFT.md` | A drafted issue for anomalyco/rune asking for a public retained write API. **Not posted.** |
| `bench/` | Every bench and driver used, as single files to drop into each repo's `examples/` (see below). |
| `patches/` | The rust-solid patches the glue and jfb benches were built against (see below). |

PERF.md and RUNE.md link to `rune-perf.diff` and `rune-retained.diff`. Those two diffs are now the
two code commits on this branch, so the links don't resolve here.

## What's done

### Findings, in short

- **Exact and solid-model are the same architecture from opposite ends.** Both compile a checked UI
  language to a plan that is pure data and run it on JS and Rust runtimes. Exact starts from hosts
  (Taffy layout, native presenters, motion, delivery). solid-model starts from semantics (Solid 2's
  transitions, optimistic writes and held async, checked against Solid JS by fuzzing).
- **Rune is a renderer and host stack**: wgpu, Parley text, Taffy layout, springs, winit. Its state
  model is rebuild-and-diff with `memo`, and it rejects signals on purpose. It has no plan or
  compiler. It complements rust-solid as the missing layout and window layer.
- **Micro-workloads** (counter, fanout, select, filter, mount): unpatched rust-solid was 2 to 4x
  slower than Exact's commit above one node. After eleven local fixes it was within about 1.1x on
  fanout, filter and mount, and 1.4x on select. A single signal write stays about 2x faster than Exact.
- **js-framework-benchmark, keyed, headless** (all nine operations, jfb's own warmups, checked
  before and after each run): patched rust-solid beats Exact's commit on eight of nine operations,
  by up to 7.8x, because Exact rebuilds lists as values while rust-solid's store writes in place.
  Once layout is included, Taffy dominates every runtime, and Rune's `virtual_rows` is the fastest
  configuration by far (1.9 to 6.5 ms for every 1,000-row operation, 10 ms to create 10,000 rows).
- **The glue** (rust-solid owns state and tree, Rune shows it): a full rebuild is slower than plain
  Rune. Windowed ("lazy") rows plus the retained write path beat Rune's virtual rows only where
  the change is text in place: jfb update 2.42 ms against 3.38, select 1.66 against 2.63, and the
  micro select at 128 µs against 235. Every structural change still runs Rune's view and loses by
  1.2 to 2.3x. Retained `insert`/`remove` is what those columns need.
- **Two findings for the codemod**: text that sits beside element siblings is replaced on every
  change in a fine-grained host (a structural change, not a text write), and plan v2 has no
  `createSelector`, so select touches every row.

### The two code commits on this branch

1. **`Find a reused memo's span by binary search`** (`crates/rune/src/memo_reuse.rs`). A commit that
   reused n memos inside an outer memo scanned every reused span per memo, which was O(n²). It now
   sorts spans by old start and binary searches, falling back to the scan when the nearest span
   doesn't contain the memo. Found while benching memo'd rows.
2. **`Add a retained write path for hosts with their own tree`** (new `runtime/retained.rs`, small
   edits to `lib.rs`, `runtime.rs`, `runtime/frame.rs`, `runtime/commit/relayout.rs`). API:
   `rt.retained()` returns a cloneable `Retained` handle with `set_text(key, text)`,
   `structure_changed()` and `landed()`. Pair writes with `Tx::live_only` in the handler. The
   next commit writes text into committed nodes, dirties their layout and reuses the resize path,
   with no view, flatten or diff. Writes land only on plain mounted, non-exiting text (no spans,
   links, input or live text). A write to a key that isn't mounted is dropped, so the view must
   build what the host shows when it mounts. Anything else runs the view.

Fixes made to the retained path while testing it, all in commit 2: it releases the old text layout
as the diff does (without that, the layout cache grew with every new string and a counter click
went from 49 to 84 µs over 50,000 clicks), and it handles `virtual_rows` replacing a row's key with
the list's key.

### The rust-solid side

- `patches/rust-solid-perf.diff`: the eleven bench fixes, against `solid-model` at `7186e52`.
  **Superseded** by the newer rust-solid performance branch (`rust-solid-perf`, on `solid-model`
  `f5418ad`), whose first commit reapplies these eleven fixes and which has gone much further
  since. It's here only so the glue can be rebuilt exactly as benched.
- `patches/rust-solid-glue.diff`: what the glue needs from rust-solid. A `Slice` expression for
  windowed lists, `App::set_signal` and `value`, version stamps in `TreeRenderer`, and a log of
  host writes (`HostWrite`) that the glue forwards to Rune. Applies on `7186e52` plus the perf diff.
- `patches/rust-solid-jfb.diff`: applies on top of the glue diff. Accepts an undefined array
  constructor in `species()` (ECMAScript's ArraySpeciesCreate does), which a filter inside a store
  write hit, and adds `App::store_set` for host-side `setStore` writes.

## What was tested, and what wasn't

Tested:

- `cargo check -p rune --all-targets` at this branch's code head passes (three warnings, all
  present on `dev` already).
- Rune's integration tests with both Rune changes applied, on a Linux VM with no GPU: 374 of the
  406 that build on Linux pass. 29 need a GPU. The other 3 (an allocation count, emoji link boxes,
  a clipboard menu) fail the same way on unpatched `dev`.
- The glue's own check (`GLUE_CHECK=1`): each variant, retained ones included, is clicked 60 times
  beside a plain Rune view of the same workload, and every text on screen matches after every
  click, on all four update workloads.
- jfb: every runtime passed the table check (every row's marker, id and label against a model)
  before and after every timed operation.
- rust-solid's own suite (132 tests) and 12 seeds of the differential fuzzer passed with the
  eleven perf fixes.

Not tested:

- Rune's in-crate unit tests with the changes: compiling them ran out of memory on the earlier VM.
  A second try for this handoff hit the same limit; see the end of this file.
- Anything with a GPU or a window: all benches are headless, and nothing paints.
- macOS and Windows.
- The retained path with rich text, inputs, links, or any structural write (none exist yet).
- The glue against the newer rust-solid performance branch. The glue diff targets `7186e52`.
- The JS side of the differential fuzzer for the perf fixes.
- Exact was not rebuilt for the jfb port beyond its own example; REPORT.md is a reading, not a run.

## How to run the benches

All numbers in PERF.md come from one 4-vCPU Linux VM with rustc 1.97.0. Rune links
`libxkbcommon` even headless, so install `libxkbcommon-dev` first. Rune examples use the fork's
`perf` profile; everything else uses `release`. Each bench file says at its top how it is run.

**Rune alone** (`bench/rune-bench.rs`, five micro-workloads; `TALL=1` for a 400 x 100,000 window):

```sh
cp handoff/rune-perf/bench/rune-bench.rs crates/rune/examples/solid_bench.rs
cargo run --profile perf -p rune --no-default-features --features embedded-fonts --example solid_bench
```

**rust-solid alone** (`bench/rust-solid-bench.rs`, `bench/rust-solid-prof.rs`): the perf diff
adds these as `examples/bench.rs` and `examples/prof.rs` in `rust-solid/crates/runtime`.
`cargo run --release --example bench` there. For instruction counts per operation,
`bench/ir.sh <binary> <workload> <n>` runs callgrind at n and 2n and prints the difference over n.

**Exact** (`bench/exact-bench.rs`, `bench/jfb-exact.rs`): in a clone of devagrawal09/exact at
`49b5625e`, copy them to `runner/examples/bench.rs` and `runner/examples/jfb.rs` (with
`jfb-common.rs` beside it as `jfb_common.rs`), then
`cargo run --release -p exact-runner --example jfb -- [op|all] [reps]`. `EXACT_LAYOUT=1` adds Taffy.

**The glue** (`bench/rune-glue.rs`, `bench/jfb-glue.rs`):

1. Check out reactivescript `solid-model` at `7186e52` and apply, in order,
   `patches/rust-solid-perf.diff`, `patches/rust-solid-glue.diff`, `patches/rust-solid-jfb.diff`.
2. In this Rune branch, add dev-dependencies to `crates/rune/Cargo.toml`:
   `rust-solid = { path = "<reactivescript>/rust-solid/crates/runtime" }` and `serde_json = "1"`.
   Keep that local; it can't be committed with a path into another checkout.
3. Copy `rune-glue.rs` to `crates/rune/examples/solid_glue.rs`, `jfb-glue.rs` to
   `examples/jfb_glue.rs`, and `jfb-common.rs` and `jfb-solid-plan.rs` beside them as
   `jfb_common.rs` and `jfb_solid_plan.rs` (they're included with `#[path]`).
4. `cargo run --profile perf -p rune --no-default-features --features embedded-fonts --example solid_glue`.
   Modes are environment variables: `GLUE_MEMO`, `GLUE_VIRTUAL`, `GLUE_LAZY`, `GLUE_RETAINED`,
   and `GLUE_CHECK` to compare against plain Rune instead of timing.

**jfb, all runtimes**: build the four jfb examples (`jfb_rune` from `jfb-rune.rs`, `jfb_glue`,
rust-solid's `jfb` from `jfb-rust-solid.rs`, Exact's `jfb`), then

```sh
RS=<rust-solid jfb binary> RU=<rune target/perf/examples> EX=<exact jfb binary> \
  bench/jfb-run.sh 5 results.jsonl
python3 bench/jfb-table.py results.jsonl          # --spread for interquartile ranges
```

`jfb-run.sh` interleaves: each round visits every operation, and within it every runtime in random
order, each as a fresh process taking 3 samples. Five rounds give 15 samples per cell. Each binary
also runs alone as `<binary> [op|all] [reps]`; `RUNE_MEMO`, `RUNE_VIRTUAL` and `GLUE_LAZY` pick
variants. The raw samples behind PERF.md's jfb table are kept outside this repo.

## Open decisions

1. **Post the Rune issue draft?** Recommendation: yes, after one more pass. The numbers and the two
   traps it names are solid, and asking for the API shape before writing `insert`/`remove` avoids
   building something upstream won't take. The draft offers to share the patch, which is now
   commit 2 here.
2. **Propose `Retained` upstream as a PR?** Recommendation: not yet. Open the issue first. The patch
   is text-only, and the structural half is where most of the remaining cost is.
3. **Which rust-solid perf line is canonical?** Recommendation: the newer `rust-solid-perf` branch
   on `f5418ad`. It already contains the eleven fixes and is far ahead. Rebase the glue diff onto
   it rather than keeping `7186e52` alive.
4. **`App::mount` taking `Rc<Plan>`** instead of a `Plan` by value. It would remove a clone of the
   whole plan on every mount (a measurable share of mount time). Recommendation: do it; it's a
   small API change while rust-solid has no outside users.
5. **Select's remaining gap to Exact** is Solid's scheduler doing what it does for 1,000 render
   effects, not the plan interpreter. Recommendation: add a `createSelector`-style plan op, which
   is what Solid apps write anyway, rather than changing the scheduler.

## Next steps

1. Rebase `rust-solid-glue.diff` and `rust-solid-jfb.diff` onto the newer rust-solid performance
   branch and rerun `GLUE_CHECK` and jfb.
2. Add retained `insert`, `remove` and `move_before` to Rune, splicing the node arena and re-syncing
   only the changed parent's layout children, with exits handled as a keyed diff would. This is
   what jfb's create, replace, swap, remove, append and clear columns need.
3. Paint-only style writes (`set_style`) that skip layout, and a Taffy fast path for a column of
   fixed-height children. Layout is about half of a full-list select click.
4. A debug check for retained writes, like the one Rune has for `Tx::live_only`: run the view now
   and then and warn on disagreement or on a write to a key that isn't mounted.
5. A `createSelector` plan op in plan v2, then rerun jfb select.
6. An async workload in the micro-bench (a mutation with a data source in Exact, an action with an
   await in plan v2).

## Test run for this handoff

On Nov 17, on a 4-vCPU, 15 GB Linux VM with this branch's two code commits: `cargo check -p rune
--all-targets` passed. `cargo test -p rune --lib` did not run: rustc was killed while compiling the
crate's unit-test build (SIGKILL, out of memory), the same failure as the earlier VM. Rune's unit
tests with these changes still need a machine with more memory.
