# Where anomalyco/rune fits beside Exact, ReactiveScript and solid-model

Read on 2026-11-10 at `anomalyco/rune` `dev` `29aacb68`. Dev's fork `devagrawal09/rune` has one branch, `dev`, at the
same commit. Read-only: README, PLAN.md, the core crate's README, research notes and source. Nothing was built or run
(see "Benchmark" below). This builds on [REPORT.md](REPORT.md) and doesn't repeat it.

## Verdict

**Rune is a renderer and host stack, not a reactivity model or a compiler.** It sits where Exact's kernel and hosts sit,
and it is a third candidate for solid-model's missing slice 4 (layout plus a native window), beside Blitz and Masonry.
It doesn't overlap with rxc or rust-solid at all: it has no plan, no language and no fine-grained graph, by choice.

## What it is

- **Who and how big.** Anomaly, the team behind OpenCode. Kit Langton wrote 1,603 of 1,918 commits since 2026-10-22,
  then Luke Parker 190 and Dax Raad 79. About 320k lines of Rust: `rune` 194k, `rune-kit` (components) 42k, plus
  mermaid, motion, effects, Luau, i18n and a CPU reference renderer. MIT. Pre-release: apps pin `dev` revs, and the
  repo takes issues, not pull requests.
- **Goal.** A Rust GPU UI framework that feels like SwiftUI on iOS at 120 Hz, with one look on every platform. The
  flagship test is recreating OpenCode Desktop (the Electron app) and beating it.
- **Stack.** wgpu with one instanced "uber-quad" pipeline and one draw call a frame; Parley text with its own
  rasterisers tuned to match CoreText, DirectWrite and Chromium; Taffy on Rune's own node arena; analytic springs on
  every property, layout edges included; winit shells. macOS, Windows and Linux now, an experimental WebGPU canvas,
  iOS and Android planned.
- **Agent loop.** Every app runs headless with a virtual clock: `--snap` to PNG, `--tree` to JSON, scripted input,
  motion traces checked by a motion linter, and an in-process `Runtime` with no window or GPU.

## Its reactivity and async model

- **Rebuild and diff, not signals.** One app state `S`. `fn view(&S, &mut Cx<S>) -> El` re-runs after each queued
  handler (`on_press(|s: &mut S| ..)`), and the keyed `El` tree is diffed against the last one. `memo(id, version, ..)`
  skips a subtree, and `virtual_rows` builds only mounted rows. PLAN.md describes typed `Model<T>`s whose reads
  subscribe; the code doesn't have them yet.
- **Signals were rejected on purpose** (PLAN §4.8, research 06): state is lost on hot patch, and a read outside a
  tracking scope fails silently. Its own research 38 then names the cost: skipping an arbitrary view helper without
  `memo` "would require reactive state handles or a view macro/compiler rewrite". That is exactly what Solid and RX
  supply.
- **Animation is never state.** A view states targets; tracks animate between them, and the view never re-runs for
  an animation frame. Neither Exact nor rust-solid separates motion from state like this.
- **Async is basic.** `Latest<T>` runs a request on a worker, newest wins, with debounce. Modes are explicit state
  machines whose `Attempt` tokens drop stale answers. There's no transition, held frame, optimistic write or
  loading boundary: about Exact's level, well behind Solid 2.
- **No compiler/runtime split.** Views are compiled Rust. Live code comes from subsecond hot-patching (about 1 s),
  live literals and tokens, and `rune-luau`: sandboxed Luau VMs whose `render` builds `El`s, memoised on a version,
  with time and heap limits and hot-state save and load. A declarative, A2UI-shaped catalog for generated UI is
  researched, not built.

## Placed beside the others

| | Exact | ReactiveScript (`rxc`) | solid-model (rust-solid) | Rune |
|---|---|---|---|---|
| Layer | Language, plan, two runtimes, layout, hosts | Language and compiler onto Solid | Semantics, plan v2, Rust runtime, renderer trait | Renderer, text, layout, motion, shells |
| Reactivity | Static dependency table over a plan | Solid 2, checked at compile time | Port of Solid 2's graph, fuzzed against Solid | Re-run view from `&S`, keyed diff, manual `memo` |
| Async | Newest wins, atomic commits | Solid 2 plus static checks | Solid 2 lanes, transitions, optimistic | `Latest<T>`, attempt fencing |
| Live code | Signed plans | — | `eval` to plans | Hot patch, Luau VMs |
| Pixels | Native controls plus Taffy kernel | Solid's DOM | Recording tree, no layout | Own GPU renderer, Taffy, springs |
| Oracle | Its own JS runner, real apps | Solid | Native Solid, generated programs | CPU reference renderer, CoreText, motion linter |

- **vs Exact: the closest neighbour.** Both own layout, text and hosts, and both are agent-first with headless
  snaps, trees and scripts. Exact adds a language, a plan and native platform controls; Rune draws everything itself
  and keeps views in plain Rust.
- **vs rxc: orthogonal.** Nothing to compare.
- **vs rust-solid: complementary.** rust-solid has the reactive core and a `Renderer` trait with nothing under it.
  Rune has everything under it and nothing above it. Rune's public seam is "hand me an `El` tree", not
  create/insert/set_prop, so an adapter would keep rust-solid's retained tree, rebuild an `El` from it each turn, and
  wrap each Solid-owned subtree in `memo(key, version)`, bumping the version only where rust-solid wrote. Rune's diff
  then walks only dirty subtrees. A lower-level path would need Rune's node arena, which isn't public.

## Worth borrowing

For solid-model:

1. **Motion as plan data.** Let a plan prop name a spring token and have the renderer own the track, so the
   reactive graph never runs per frame. Rune's per-edge layout springs and presence (exit, then unmount at opacity 0)
   are the reference.
2. **An in-process headless runtime as the test driver.** `Runtime::new(state, view, size, env)`, `frame(dt)`,
   `find(key)`, `pointer_down/up`, `--tree`, `--snap`. This is the concrete version of Exact idea 5 in REPORT.md.
3. **A CPU reference renderer** (`rune-ref`) differential-tested against the GPU path, with tolerances per channel.
   The same oracle idea gives slice 4 a gate for layout and paint.
4. **Luau's sandbox limits** (watchdog, heap cap, version memo, hot-state save) as a checklist for `createEval`.

For Rune, the other way: a Solid front end. Fine-grained reads answer its research 38 problem (automatic skipping
without `memo`), and RX's checker refuses untracked reads at compile time, which removes the silent-failure reason
Rune gave for rejecting signals. Hot patching keeps state too when the state lives in plans.

## Risks of building on it

Pre-release with rev pins and no outside PRs; scope its own PLAN calls "years of work for a human team"; tuned
first for Kit's and OpenCode's surfaces; the PLAN still differs from the code (render thread, `Model<T>`,
AccessKit, inspector socket). Fine for a slice 4 experiment, not yet a dependency.

## Benchmark

Rune has a runtime that fits the five workloads in [PERF.md](PERF.md): `Runtime::new` plus `frame`, headless, the
way its own `interaction_cost` example drives it. One frame after a click runs the handler, re-runs the view, diffs,
lays out with Taffy and shapes changed text, so it compares with Exact's "commit + layout" column. The bench is
[bench/rune-bench.rs](bench/rune-bench.rs), built from `devagrawal09/rune`. Results are in [PERF.md](PERF.md) under
"Rune". In short: written as one flat list, Rune is 1.4x to 4x slower than Exact with layout, and `memo` doesn't help.
Written with Rune's own `virtual_rows`, it gets 2.4x to 5x faster (select 232 µs, filter 713 µs on a 400 x 800
window), because only rows near the viewport are built and laid out.

## The glue, prototyped

The adapter from "vs rust-solid" above now exists as a local prototype ([bench/rune-glue.rs](bench/rune-glue.rs),
with [rust-solid-glue.diff](rust-solid-glue.diff) on rust-solid and [rune-retained.diff](rune-retained.diff) on
Rune). Details and the full table are in [PERF.md](PERF.md) under "Glue".

- **Through the `El` seam alone, it costs about rust-solid plus Rune** and never beats plain Rune: Rune still re-runs
  the view, diffs and lays out every child each frame.
- **Lazy rows plus a retained write path do beat it.** With rust-solid creating only the rows Rune shows, and a
  small Rune patch that writes changed text straight into the committed nodes (no view, flatten or diff), select
  takes 128 µs against 235 for plain Rune with `virtual_rows` and 2,087 for the first glue. Every variant shows the
  same text as plain Rune after each click.
- **What's left is mostly Taffy and structure.** One changed leaf in a 1,000-child column reruns flexbox over all
  children, and inserts and removes (filter) still go through the view.

The issue draft asking Rune for a public version of this is [RUNE-ISSUE-DRAFT.md](RUNE-ISSUE-DRAFT.md), not posted.
Two Rune changes are saved as diffs, nothing pushed: the retained path, and a quadratic search in memo reuse turned
into a binary search ([rune-perf.diff](rune-perf.diff), −18% on memo'd select).
