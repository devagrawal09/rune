# Exact vs rust-solid: performance (Nov 10, 2026)

## Status

Both halves ran on the same VM, back to back. The Exact half was built from
Dev's fork (devagrawal09/exact, HEAD 49b5625e), with the bench dropped in as
`runner/examples/bench.rs` in a scratch clone. Nothing was pushed.

To build, the bench needed one fix: the number variant is `Value::Number`, and
state is read with `Runner::slot`. Both bench files here are the fixed versions.

**Update (round 2 of profiling, below):** with eleven local fixes to rust-solid,
it is now within about 1.1x of Exact's commit on fan-out, filter and mount, and
1.4x on select. The upshot paragraph that follows describes the unpatched
code.

**Update (js-framework-benchmark, Nov 11, last section):** on jfb's nine keyed
operations, patched rust-solid beats Exact's commit on eight of nine, by up to
7.8x, because Exact rebuilds its lists as values. Once layout is included,
Taffy dominates every runtime, and Rune's virtual rows are the fastest by far.

**The upshot (unpatched rust-solid):** for anything above one node, Exact's commit is 2 to 4 times
faster than rust-solid. A single signal write is the exception, where
rust-solid is twice as fast. Exact still wins on lists with Taffy layout
included.

## What the benchmark measures

There are five workloads, written once for each runtime. Both run headless and
in-process, in a release build with thin LTO, codegen-units=1 and panic=abort
(Exact's own release profile).

Each click is timed from the click to a settled tree. The figure is the best of
five runs, and every run checks the final value so a broken workload can't
report a fast time.

| Workload | What a click does |
|---|---|
| counter | One signal +1, one text node updates (100k clicks) |
| fanout | One signal +1 drives 1,000 derived values, each shown in its own text (2k clicks) |
| select | 1,000-row keyed list; the selected id +1, so every row's "> " marker recomputes (2k clicks) |
| filter | 1,000-row list; toggle "all" / "even only", so 500 rows are removed or created (1k clicks) |
| mount | Build the 1,000-row list from a prebuilt plan, flush, tear down (100 mounts) |

- **rust-solid:** `App::mount` on a plan v2 built in code, then `click` through
  the delegated handler and one `turn()`. Source: `bench/rust-solid-bench.rs`,
  run as an example of `fork/rust-solid/crates/runtime`.
- **Exact:** the same five as Contract source, compiled with `contract::compile`,
  then `Runner::boot` with `Kernel::with_monospace()` and `dispatch(button, Press)`.
  Each workload is timed twice: the commit alone (plan VM plus kernel node
  apply), and the commit plus `compute_layout` on the root (Taffy). Source:
  `bench/exact-bench.rs`, run as an example of `exact/runner`
  (`cargo run --release --example bench -p exact-runner`).

## Numbers (4-vCPU cloud VM, Linux, rustc 1.97.0)

Microseconds per click (per mount for the last row), best of five. These are
from two back-to-back rounds; the rounds agreed within about 5%.

| Workload | rust-solid | Exact commit | Exact commit + layout | Exact commit vs rust-solid |
|---|---:|---:|---:|---|
| counter | 1.6 | 3.2 | 8.6 | rust-solid 2x faster |
| fanout (1,000 memos) | 1,200 | 590 | 1,700 | Exact 2x faster |
| select (1,000 rows) | 1,140 | 340 | 570 | Exact 3.3x faster |
| filter (±500 rows) | 5,900 | 1,490 | 2,040 | Exact 4x faster |
| mount (1,000 rows) | 7,200 | 2,920 | 4,120 | Exact 2.5x faster |

An earlier lone rust-solid run measured fanout at 888 and select at 826. The
VM's speed drifts, so compare only numbers from the same round.

What it means:

- **Fixed cost per commit:** Exact pays about 3 µs for a commit; rust-solid's
  click and flush cost about 1.6 µs. A commit is a transaction (checkpoint,
  dirty sweep, kernel apply), so a lone write costs more there.
- **Cost per node:** Exact is much cheaper. That is about 0.3 to 0.6 µs per
  derived value or row against rust-solid's 1.1 to 1.2 µs, and it is the
  number that matters for real screens. This fits the design difference: Exact
  sweeps a static dependency table over a compiled plan, while rust-solid
  builds a dynamic graph of `Rc<RefCell>` nodes and looks names up while
  evaluating. That is a likely cause, not a profiled one.
- **Creating and removing rows:** Exact is cheapest relative to rust-solid here
  (filter 4x, mount 2.5x). So rust-solid's per-row ownership and renderer setup
  is the first place to look.
- **With layout:** Exact still beats rust-solid's no-layout numbers on select,
  filter and mount. Only fanout (1,000 sibling texts, all re-laid out) and
  counter come out slower. For a native plan runtime, rust-solid's per-node
  overhead is the gap, not reactivity semantics.

## Fairness caveats (read before comparing)

- **Layout.** Exact's commit updates a real node arena. Its frame includes a
  Taffy CSS layout, and its hosts paint from that. rust-solid renders into a
  recording tree with no layout. The "commit" column is the like-for-like one;
  "commit + layout" is the cost of a real frame that rust-solid doesn't do yet.
- **Same app, different idioms.** The select row in Contract is one `text` with a
  template. In plan v2 it is a `p` with one concatenated text node. Lists are
  keyed by `r.id` in Exact and by identity in plan v2. Neither side uses a
  selector primitive, so both recompute all 1,000 rows on select.
- **Not measured.** Async work isn't in either half. Plan v2 only awaits inside
  actions, while Exact goes through resources and mutations with a DataSource,
  so a fair async case needs more design. rust-solid's request list is a Vec
  scanned on every settle, which would skew long async runs.
- **Exact's own benches** in `bench/` are device-level (iOS scroll fps, launch to
  first frame against SwiftUI, Expo and GPUI). They don't compare with a headless
  runtime number.
- **Both benches live only in scratch clones** (the Exact one in a clone of Dev's fork). Nothing was pushed.

## What's left

1. Profile rust-solid's fanout and filter: per-memo evaluation and per-row
   create/dispose are where the 2 to 4x gap lives.
2. Add an async case. Exact would use a mutation with a DataSource, and plan v2
   an action with an await, both resolved in the bench. rust-solid's request
   list should become a map first, because it is scanned on every settle.
3. Optional: a JS leg, Exact `rt.js` against Solid via `solid-plan.ts` under
   Node. It needs the fork's `solid-next` sibling packages, which aren't in this
   checkout.

## Rune (added later on Nov 10)

Built from Dev's fork (devagrawal09/rune, `dev` at `29aacb68`, the same commit
as upstream), with `bench/rune-bench.rs` copied in unchanged as
`crates/rune/examples/solid_bench.rs` in a scratch clone. Built with
`--profile perf --no-default-features --features embedded-fonts`. Linking
needed the system `libxkbcommon-dev` package (winit links it, even headless).
Nothing was pushed.

It drives Rune's headless `Runtime` (no window, no GPU): a click, then one
`frame`, which runs the handler, re-runs the view, diffs by key, lays out with
Taffy and shapes changed text. So it compares with Exact's **commit + layout**
column. Every run checks that every click landed.

Microseconds per click (per mount for the last row), best of five. Two rounds
agreed within about 7%.

| Workload | Rune (commit + layout) | Exact commit + layout | rust-solid (no layout) | Rune vs Exact + layout |
|---|---:|---:|---:|---|
| counter | 27 | 8.6 | 1.6 | Exact 3x faster |
| fanout (1,000 values) | 2,360 | 1,700 | 1,200 | Exact 1.4x faster |
| select (1,000 rows) | 2,200 | 570 | 1,140 | Exact 3.9x faster |
| filter (±500 rows) | 5,100 | 2,040 | 5,900 | Exact 2.5x faster |
| mount (1,000 rows) | 17,000 | 4,120 | 7,200 | Exact 4.1x faster |

With each row wrapped in `memo(id, selected)`, Rune's own way to skip
unchanged subtrees, select came out at 2,500 and filter at 5,300: no faster,
slightly slower.

What it means:

- **Rebuild-and-diff costs what you'd expect.** Rune re-runs the whole view and
  diffs 1,000 rows on every click, so each workload pays for every row, changed
  or not. That is the cost its own research note 38 describes, and why
  fine-grained reads matter. Even with real layout and text, Rune is close to
  rust-solid's no-layout figure on filter, and about 2x on fanout and select.
- **`memo` didn't help here.** The root still builds 1,000 children each click,
  and each memo has a formatted id. The likely cause is that the per-row work
  left is the diff and apply, not the build; that's a guess, not a profile.
- **Mount is Rune's weakest case** at 17 ms for 1,000 rows, about 4x Exact.
- **A frame does more than a commit.** Even the counter pays about 27 µs, which
  includes Rune's frame work (pointer handling, animation sampling, scene
  apply). Exact's figure is a dispatch plus a layout call.

Caveats on top of the ones above:

- **Not the same round.** The Exact and rust-solid figures came from an earlier
  session's VM (rustc 1.97.0). Rune ran on a fresh 4-vCPU Xeon VM with rustc
  1.99.1. Earlier runs drifted up to about 35% between rounds, so treat the
  ratios as rough.
- **Real text.** Rune shapes text with Parley through its caches. Exact's bench
  used `Kernel::with_monospace()`, and rust-solid lays out nothing.
- **Same app, Rune idioms.** Rows are keyed `text` elements; there is no
  selector primitive on any side.

### Glue: rust-solid under Rune, lazy rows and a retained write path (Nov 10, late)

Dev asked for a prototype of the glue, then for a lower-level write path inside
Rune itself. Everything here is local and unpushed; the diffs and sources are in
this folder.

**Builds.** Rune: Dev's fork `29aacb68` plus [rune-perf.diff](rune-perf.diff)
and [rune-retained.diff](rune-retained.diff). rust-solid: solid-model `7186e52`
plus the eleven-fix [rust-solid-perf.diff](rust-solid-perf.diff), plus
[rust-solid-glue.diff](rust-solid-glue.diff).

**The glue** ([bench/rune-glue.rs](bench/rune-glue.rs)). rust-solid's `App` owns
the state and its retained `TreeRenderer` tree. The button's handler calls
`app.click` and `app.turn`. Modes:

- **rebuild:** each frame, walk the tree and build an `El` for everything.
- **memo per node:** each element is wrapped in `memo(n{id}, version)`, using
  version stamps that `TreeRenderer` now keeps.
- **virtual:** an element with more than 64 children becomes Rune's
  `virtual_rows`, so Rune builds and lays out only the rows in its window.
- **lazy:** the list's plan iterates `rows.slice(winStart, winEnd)`, and Rune's
  `on_visible_rows` sets those two signals, so rust-solid only creates and
  updates the rows Rune shows (78 of 1,000 here).
- **retained** (any of the above plus `GLUE_RETAINED=1`): `TreeRenderer` logs its
  writes, and the handler forwards text writes to Rune's new `Retained` handle
  and calls `Tx::live_only`. Rune writes the text into the committed nodes,
  dirties their layout and lays out again the way a resize does, with no view,
  flatten or diff. A child-list change marks the commit structural, and the
  view runs as before.

**The Rune patch** ([rune-retained.diff](rune-retained.diff), about 125 lines):
a new `runtime/retained.rs` (`Runtime::retained()`, `Retained::set_text`,
`structure_changed`, `landed`) and a `CommitKind::Patch` that reuses the
`Relayout` path. Writes land only on plain text (no spans, links, input or live
text) that is mounted and not exiting; a write to a key that isn't mounted is
dropped, since the view builds that row from the host when it shows it; anything
else runs the view.

**Checks.** `GLUE_CHECK=1` boots each variant beside a plain Rune view of the
same workload, clicks both 60 times, lets both settle, and compares every text
on screen after each click. All modes, retained ones included, match on all
four update workloads. Rune's own tests, with both Rune diffs applied: 374 of
the 406 integration tests that build on Linux pass. 29 need a GPU this VM
doesn't have, and the other 3 (an allocation count, emoji link boxes, a
clipboard menu) fail the same way on unpatched Rune. Rune's in-crate unit tests
weren't run: compiling them ran out of memory here, twice.

**Numbers.** One VM, one session, a 400 x 800 window, best of five, µs per click
(per mount for the last row).

| Workload | Rune plain | Rune virtual | Glue rebuild | Glue memo | Glue virtual | Glue lazy | Retained | Retained + virtual | Retained + lazy |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| counter | 28 | – | 35 | 38 | 36 | 34 | 33 | 33 | 33 |
| fanout | 1,714 | 495 | 3,290 | 4,157 | 1,230 | 1,221 | 2,922 | 1,324 | 1,336 |
| select | 1,197 | 235 | 2,087 | 1,722 | 840 | 235 | 1,321 | 659 | **128** |
| filter | 1,591 | 688 | 4,842 | 5,273 | 2,565 | 2,223 | 4,812 | 2,738 | 1,831 |
| mount | 14,351 | 6,193 | 19,225 | 20,514 | 10,003 | 15,073 | 19,803 | 10,847 | 11,617 |

For reference, not from this session: rust-solid alone with the same patch, on
this VM earlier tonight, 1.7 / 606 / 443 / 1,661 / 3,554 (no layout); Exact's
commit + layout from round 1 on another VM, 8.6 / 1,700 / 570 / 2,040 / 4,120.

Retained writes skipped the view on every fanout and counter click, and on
every select click that changed something on screen (in the full list, clicks
past row 1,000 change nothing; in lazy mode, the selected row is usually outside
the window, so most clicks write nothing). Filter is structural, so it never
skips the view.

**What it shows.**

1. **Retained + lazy is the first combination that beats plain Rune.** Select
   takes 128 µs: 1.8x faster than plain Rune with `virtual_rows` (235), 16x
   faster than the first glue (2,087), and below Exact's commit + layout figure.
   rust-solid updates only the window's rows, and Rune writes and lays out only
   the two strings that changed.
2. **Without lazy rows, retained helps less.** Full-list select drops 37%
   (2,087 to 1,321) and fanout 11%. Callgrind on full-list select: 17.4M
   instructions per click, 10.6M in Rune's commit, 8.5M of it in Taffy. When one
   leaf of a 1,000-child flex column changes, Taffy reruns flexbox over every
   child, so dirty-subtree layout is limited by the flex algorithm, not Rune's
   bookkeeping.
3. **Fanout doesn't move with retained writes** (1,336 against 1,221 lazy).
   Fanout's 1,000 values aren't windowed in the plan, so rust-solid recomputes
   all of them, and Rune still lays out the window.
4. **Structural changes need the next piece.** Filter with retained + lazy is
   1,831 (18% under lazy alone), but it still runs the view and is 2.7x plain
   Rune virtual. That needs `insert`/`remove` in the retained API.
5. **Counter shows Rune's floor.** Plain Rune is 28 µs and every glue mode is
   33 to 38, so the cost there is Rune's frame, not the view.

**Two bugs found in the prototype while testing, both fixed:**

- The patch path didn't release the old text layout the way the diff does
  (`TextSystem::supersede`), so the layout cache grew with every new string and
  each commit's eviction sort grew with it: a retained counter click went from
  49 µs at 1,000 clicks to 84 µs at 50,000. Now flat at about 30.
- `virtual_rows` replaces a row's root key with the list's key, and caches the
  key list per `virtual_rows_version`. In lazy mode the key list was cached
  under a constant version, so retained writes named keys that weren't mounted
  and were silently dropped (the check caught it). The glue now keys rows by the
  key `build` gives their root and versions the key list by the window. A debug
  warning in Rune for writes to unmounted keys would catch this.

**Rune fix kept from the first round.** `MemoCache::commit_flattened` searched
all reused spans for every flattened memo (quadratic). Sorting the spans and
binary-searching cut memo'd select from about 2,800 to 2,300 µs at 100,000 tall
(−18%). It's in [rune-perf.diff](rune-perf.diff), unpushed.

**Issue draft.** [RUNE-ISSUE-DRAFT.md](RUNE-ISSUE-DRAFT.md) asks anomalyco/rune
for a public retained API (set_text, paint-only set_style, insert, remove, move,
stable handles, a debug check). Not posted.

## Profile of rust-solid (Nov 10, later)

Profiled with callgrind (perf isn't available on this VM). To get one click's
cost, run N clicks and 2N clicks and divide the difference by N, so mount and
teardown cancel out. Instruction counts are deterministic. Wall-clock time on
this VM swings by ±25% between runs. Tools: `bench/rust-solid-prof.rs` and
`bench/ir.sh`.

### What the per-node cost was (before any change)

1. **Keyed list diffing was quadratic.** JS `Map` was modelled as a `Vec` with
   a linear `find`, in two places: `keyed_map_general` (map.ts `newIndices`)
   and `reconcile_arrays` (universal.ts). In filter, `Value::eq` alone was 34%
   of all instructions, with `keyed_map_pass` at 48% and `reconcile_arrays`
   at 31% inclusive.
2. **The test renderer records every host call.** `TreeRenderer` builds a
   serde_json entry per createElement, insertNode, replaceText, getParentNode
   and so on, and keeps them forever. This is oracle machinery for fuzzing,
   not runtime cost. Still, it was about 20 to 35% of fan-out and mount, and
   the log grows without bound in a long run.
3. **The plan is interpreted on every recompute.** Each evaluation looks
   source names up in a `HashMap<String, _>` with SipHash (5% of fan-out). Each
   text node's closure deep-cloned its `Expr` tree when created (16% of mount).
   Each `get` allocates its key (`Key::name`, about 4%). Each row clones a
   `BTreeMap<String, Value>` environment.
4. **Allocation overall:** malloc and free are about 20% of every workload.

### Prototype fixes (local only, nothing pushed)

The patch is in `rust-solid-perf.diff` (6 files, +117/−65):

- A `Hash` impl on `Value` that agrees with its `eq` (objects by identity,
  strings by content). Both JS-Map stand-ins become `HashMap`s. Only `get`
  and `set` are used, never iteration, so order can't leak into behaviour.
- `TreeRenderer::set_logging(false)`, plus `App::mount_unlogged`. Logging is on
  by default, so tests and oracles are unchanged; the bench turns it off.
- `Box<Expr>` becomes `Rc<Expr>` in the plan schema (serde `rc` feature), so
  per-row closures share subtrees instead of copying them.
- An FxHash hasher for the plan's names table.

**Correctness:** all 127 of rust-solid's Rust tests pass, the same as before
the patch. That includes the plan-fuzz corpus (12 recorded seeds against
Solid's oracle output) and the slice unit fixtures. The live JS differential
fuzz (`plan-fuzz`, `fuzz`) can't run here because it needs the fork's
`solid-next` sibling packages.

### Effect

Instructions per click (per mount for mount), from callgrind:

| Workload | Before | Hash maps | + no op log | + Rc<Expr>, FxHash | Change |
|---|---:|---:|---:|---:|---:|
| counter | 20.5k | 20.4k | 17.1k | 16.7k | −18% |
| fanout | 7.88M | 7.86M | 5.28M | 5.05M | −36% |
| select | 6.12M | 6.12M | 6.22M | 6.09M | 0% |
| filter | 50.5M | 17.1M | 14.4M | 14.0M | −72% |
| mount | 58.9M | 50.0M | 43.6M | 36.8M | −37% |

Wall clock, µs, the best of three interleaved rounds in one sitting:

| Workload | rust-solid before | rust-solid after | Exact commit |
|---|---:|---:|---:|
| counter | 1.23 | 0.96 | 2.46 |
| fanout | 773 | 522 | 448 |
| select | 706 | 567 | 266 |
| filter | 4,535 | 1,854 | 1,123 |
| mount | 5,316 | 3,424 | 2,203 |

The gap to Exact shrinks:

- fanout: 1.7x → 1.15x
- filter: 4x → 1.65x
- mount: 2.4x → 1.55x
- select: 2.65x → 2.1x

### What's left, and roughly what it's worth (as of round 1; see Round 2 below)

Select now spends most of each row's recompute in three places:

- building the row's string (`text()` through `fmt`, about 15%)
- the store's get trap with tracking (`js_get`, about 11%)
- `Key::name` allocation

Possible next steps:

1. **Build strings without `fmt`, and reuse the `Rc<str>` when the text is
   unchanged.** Cheap. Worth about 10 to 15% on select and fan-out.
2. **Intern property keys when the plan loads** (`Get { key }` holds a
   prebuilt `Key`). Cheap. Worth about 5%.
3. **Cheaper row environments.** A persistent or `Rc`-shared environment
   instead of cloning a `BTreeMap<String, Value>` per row and per text closure.
   Moderate work. Worth about 10% on mount and filter.
4. **Resolve the plan before running it.** Turn names into slot indices and
   expressions into a compact form once, instead of walking the serde tree per
   recompute. This is the structural difference from Exact's precompiled plan
   and the only item likely to close select's 2x. A bigger change, and it
   touches every evaluator in `plan_v2.rs`.

None of these change Solid's semantics. They are all in the plan interpreter
and the host, not in the scheduler or the reactive graph.

## Round 2 of profiling (Nov 10, late)

Dev asked to keep going on performance. Everything is still local; nothing
was pushed. The whole patch (all eleven fixes, plus the bench and profiling
examples) is `rust-solid-perf.diff`, against solid-model at 7186e52.

### Fixes 5 to 11

5. **Literals made once.** A plan literal kept its JSON and was converted to a
   value, with a fresh string allocation, on every evaluation. It's now cached
   on the expression the first time it runs.
6. **Property keys made once.** `Get` and object fields built their `Key` from
   the name string (a parse attempt plus an `Rc<str>` allocation) on every
   evaluation. The key is now cached on the expression.
7. **Strings joined without `fmt`.** `text()` went through `Display`, and the
   result was then copied into a new `Rc<str>`. Now a join is one sized
   allocation, `"" + s` and `s + ""` return `s` itself, and a text accessor
   hands back the string it was given instead of copying it.
8. **Row scopes shared, not copied.** A row's or closure's locals were a cloned
   `BTreeMap<String, Value>`. They are now a persistent list (newest first),
   so a copy is one `Rc` clone and adding a local copies nothing. Lookup sees
   the newest binding of a name, as the map did.
9. **FxHash for identity tables.** The function table (read on every call) and
   the store's raw-object maps used SipHash on `u64` ids, as did the key maps
   from fix 1. They now use one shared FxHash.
10. **Dense arrays.** JS arrays kept their elements in a `BTreeMap<u32, Value>`,
    so building or reading a 1,000-element array cost a tree insert or lookup
    per element. Elements are now a `Vec<Option<Value>>` (a hole is `None`),
    with a sorted map only past a gap of more than 64, so `a[1e9] = x` stays
    cheap and keys still come out in ascending order.
11. **O(1) child insert and remove in the test host.** `TreeRenderer` kept
    children in a `Vec` and found a child by linear search, so removing 500
    rows was quadratic. Children are now a doubly linked sibling list, as in a
    real tree. This is host code, not the runtime; a DOM or native host
    already has O(1) removal.

Fix 4 in the list above, "resolve the plan before running it", turned out not
to need a separate compile pass. Fixes 5, 6 and 8 cache what the walk was
rebuilding, and after them the plan walk is about a fifth of select's
instructions. Name lookups for sources (one Fx hash per read) are the only
part left that a slot table would remove, and they're a few percent.

**Correctness:** all 132 tests in the rust-solid workspace pass (`cargo test
--release`; the 127 in round 1 was a narrower count of the same suite). That
includes the 12 recorded plan-fuzz seeds checked against Solid's oracle
output. The live JS differential fuzz still can't run here.

### Effect

Instructions per click (per mount for mount), from callgrind:

| Workload | Unpatched | Round 1 (fixes 1–4) | Round 2 (fixes 1–11) | Change from unpatched |
|---|---:|---:|---:|---:|
| fanout | 7.88M | 5.05M | 4.84M | −39% |
| select | 6.12M | 6.09M | 3.65M | −40% |
| filter | 50.5M | 14.0M | 8.34M | −83% |
| mount | 58.9M | 36.8M | 28.3M | −52% |

Wall clock, µs, best of interleaved rounds in two sittings (this VM is noisy;
fan-out moved between 430 and 570 from round to round):

| Workload | rust-solid unpatched | rust-solid round 2 | Exact commit | Gap to Exact |
|---|---:|---:|---:|---:|
| counter | 1.25 | 1.10 | 2.46 | rust-solid 2.2x faster |
| fanout | 783 | 430 | 447 | about even |
| select | 700 | 366 | 266 | 1.38x |
| filter | 4,982 | 1,268 | 1,131 | 1.12x |
| mount | 4,914 | 2,423 | 2,174 | 1.11x |

Exact's counter figure is from the first sitting; the bench's counter
workload only runs on rust-solid in later rounds.

### What's left

- **Select (1.38x).** Each click recomputes 1,000 row texts. About a fifth of
  the instructions are now the plan walk. Most of the rest is Solid's
  reactive graph doing what Solid does for 1,000 render effects: the dirty
  heap, recompute, `read_node`, dependency relinking and running the effects.
  Exact's static dependency table skips that bookkeeping. Closing the rest
  would mean changing the scheduler, not the interpreter, so it is out of
  scope for "keep Solid's semantics". A `createSelector`-style plan op would
  make select O(1) in both runtimes, but that changes the workload, not the
  runtime.
- **Mount (1.11x).** What's left is spread out: allocation (about a third),
  arena `Vec` growth as nodes are created, and the bench itself cloning the
  1,000-row seed plan for every mount, because `App::mount` takes the plan
  by value. Taking `Rc<Plan>` would remove the clone, but that's an API
  change.
- **Counter** moved little (16.7k → 18.0k instructions per click) and is
  still about 2x faster than Exact.


## js-framework-benchmark, keyed, headless (Nov 11)

Dev asked to port the standard suite and to check whether a port already
exists.

**Prior art.** No headless or native port of the keyed suite turned up. The
closest are Dioxus's `benches/jsframework.rs`, which only creates 10,000
rows into a no-op renderer, the Rust wasm entries in the browser suite itself,
and the `mogwai-js-framework-benchmark` crate, which is a wasm entry. Exact's
QUEUE.md plans a "Krausest entry" but hasn't built one.

**What was ported.** All nine keyed operations of krausest's suite, each with
jfb's own init and warmups (`benchmarksPuppeteer.ts`): create 1,000 rows
after five create/clear cycles, replace all after five creates, update every
10th row after three warmup updates, select row 2 after selecting row 5, swap
rows 2 and 999 after six swaps, remove row 4 after removing rows 9 to 5,
create 10,000 rows, append 1,000 to 1,000, and clear 1,000. The row is jfb's:
an id cell, a label link that selects, a remove link holding a span, and an
empty cell.

Each sample is a fresh runtime, the untimed init, then the timed operation.
The table must be right both before and after the timed operation: every
row's marker, id and label against a model, or for a virtual list a window
from the top that covers the viewport. Every runtime passed every check.

**Deviations from jfb, the same for all runtimes:**

- Labels come from a hash of the row id instead of `Math.random`, so every
  runtime builds the same table. They use jfb's word lists.
- The selected row shows a cell reading "danger" ("-" otherwise) instead of a
  class, because plan v2 has no attributes. That makes the table one cell
  wider than jfb's.
- Operations go through each runtime's handler path without a pointer hit
  test. Nothing paints. Data building counts, as in jfb.
- rust-solid uses `cond(r.id == sel)` in every row, since plan v2 has no
  `createSelector`. jfb's Solid entry uses `createSelector`, so here select
  touches 1,000 rows instead of 2.

**The runtimes.**

| Name | What runs | Timed region |
|---|---|---|
| rust-solid | Plan v2, latest eleven-fix patch, recording host, no layout | write or click, one `turn` |
| exact | The table as a contract, devagrawal09/exact 49b5625e | `dispatch(Press)` (plan VM and kernel apply) |
| exact-layout | Same, plus a Taffy layout at 400 x 800 | dispatch, layout |
| rune | Plain Rune view over a `Vec` of rows, keyed by id | `update`, one frame (view, diff, layout, shaping) |
| rune-memo | Same, each row `memo(id, (label, selected))` | same |
| rune-virtual | Same, rows in `virtual_rows` | same |
| glue-retained | rust-solid's tree shown by Rune; text writes through the retained patch | rust-solid, then one frame |
| glue-retained-lazy | Same, rows windowed in rust-solid by Rune's visible range | rust-solid, then frames until the window stops moving |

Same 4-vCPU VM and rustc 1.97.0 for all. Rune examples use the fork's `perf`
profile, the others `release`. `jfb-run.sh` ran 5 rounds. Each round visits
every op and, within it, every runtime in random order, as a fresh process
taking 3 samples. That gives 15 samples per cell.

**Results.** Median ms per operation. Lower is better.

| Runtime | create 1k | replace 1k | update 10th | select | swap | remove | create 10k | append 1k | clear |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| rust-solid (no layout) | 21.8 | 23.6 | **0.23** | 0.55 | **0.52** | **0.62** | **105** | 9.26 | **0.70** |
| exact (commit) | 34.2 | 43.3 | 0.49 | **0.49** | 0.89 | 0.85 | 483 | 40.3 | 5.44 |
| exact-layout | 62.3 | 82.2 | 14.0 | 13.6 | 13.7 | 14.5 | 859 | 88.9 | 7.45 |
| rune | 65.0 | 76.3 | 20.1 | 13.6 | 15.3 | 16.9 | 624 | 82.9 | 8.50 |
| rune-memo | 62.8 | 73.0 | 14.5 | 7.73 | 8.53 | 9.77 | 642 | 76.7 | 8.32 |
| rune-virtual | **4.78** | **6.48** | 3.38 | 2.63 | 2.10 | 3.08 | **9.81** | **4.10** | 1.86 |
| glue-retained | 134 | 158 | 17.7 | 11.0 | 27.0 | 33.0 | 1,276 | 160 | 13.7 |
| glue-retained-lazy | 11.0 | 14.5 | 2.42 | 1.66 | 3.04 | 3.87 | 18.6 | 4.74 | 2.87 |

Bold marks the best in each column within the two groups: no layout (first
two rows) and with layout (the rest). The interquartile range was within about
15% of the median for most cells. rust-solid was noisier, at 35 to 65% on
create, update, select and append, and the swaps were noisy everywhere (25 to
40%). Raw samples are in `bench/jfb-results.jsonl`, and `jfb-table.py
--spread` prints the spreads.

**What it changes about the earlier conclusions.**

- **rust-solid against Exact's commit flips.** The five micro-workloads had
  Exact ahead, by 1.1 to 1.4x after the eleven fixes. On jfb, rust-solid is
  ahead on eight of nine operations. The gaps are 1.6x on create, 2x on
  update, 4.6x on create 10k, 4.4x on append and 7.8x on clear. Select is
  even (0.55 against 0.49), and only there does rust-solid run 1,000 row
  effects. The difference is the data model. Exact's lists are values, so
  every jfb action builds a new list: `rows.map` over 1,000 rows to change
  100 labels or swap two rows, `concat` to append. rust-solid's store writes
  in place, and its keyed `For` reuses rows. Exact's earlier wins were
  workloads with no list writes.
- **Layout decides every number once it's in.** For a one-row change in a
  1,000-row table, Exact plus Taffy, plain Rune and the full-list glue all
  land at 11 to 20 ms. Per click, the reactive core costs 0.2 to 0.9 ms of
  that. This is the earlier Taffy finding, now on real rows: a flex column
  re-lays out every child.
- **Rune's memo now pays.** On the old single-text rows, memo rows were no
  faster. On jfb's five-cell rows with two handlers, memo cuts select, swap
  and remove by about 40%. It does nothing for creates.
- **Virtual rows are still the biggest lever, by far.** Rune with
  `virtual_rows` does every 1,000-row operation in 1.9 to 6.5 ms, and creates
  10,000 rows in 10 ms instead of 624.
- **The glue's verdict narrows.** Retained plus lazy beats Rune's virtual
  rows only where the change is text in place: update (2.42 against 3.38)
  and select (1.66 against 2.63). On anything structural it loses by 1.2 to
  2.3x: create, replace, create 10k, swap, remove, append and clear. Those pay
  rust-solid's row creation, the full El rebuild from rust-solid's tree, and an
  extra frame for the window to reach rust-solid. The full-list retained glue
  is the slowest variant (create 134 ms, twice plain Rune), so it only works
  windowed. Retained `insert`/`remove`, the next ask in the Rune issue draft,
  is what the structural columns need.

**Found along the way (rust-solid, local fixes only):**

- A filter inside a store write (the remove link's
  `rows.list = rows.list.filter(..)`) failed with "an array's constructor is
  undefined". A draft hides `constructor`, as Solid's proxy does, and
  `species()` rejected the undefined. ECMAScript's ArraySpeciesCreate treats an
  undefined constructor as a plain array. `rust-solid-jfb.diff` accepts it,
  and adds `App::store_set` for host-side `setStore` writes. It applies on top
  of `rust-solid-glue.diff`.
- A text that sits beside element siblings (the marker under `tr`, before the
  `td`s) is replaced on every change: removed and a new node inserted, as in
  dom-expressions' multi-child insert. Select then wrote 2,000 child-list
  changes, and the retained glue fell back to the full view (45 ms instead of
  11). The bench puts the marker in its own `td`. This matters for the
  codemod: text next to elements is a structural change in a fine-grained
  host.

**Sources** (in `bench/`):

- `jfb-common.rs` holds the ops, warmups, labels, model and checks.
- `jfb-solid-plan.rs` holds the plan and store writes, shared by rust-solid
  and the glue.
- One file per runtime: `jfb-rust-solid.rs`, `jfb-rune.rs` (`RUNE_VIRTUAL`,
  `RUNE_MEMO`), `jfb-glue.rs` (`GLUE_LAZY`) and `jfb-exact.rs`
  (`EXACT_LAYOUT`). Each is an example of its repo and includes the shared
  files with `#[path]`.
- `jfb-run.sh` runs the rounds, and `jfb-table.py` prints the table.

Nothing was pushed.
