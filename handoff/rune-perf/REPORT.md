# expo/exact vs ReactiveScript vs the solid-model Rust work

Compared on 2026-11-10 at these commits:

- expo/exact: `5434f432`
- solidjs/reactivescript: `main` at `607f605`
- devagrawal09/reactivescript: branch `solid-model` at `7186e52`

**The Rust side is `solid-model`.** That branch is on Dev's fork and has its own history. It consolidates
`rust-solid-main`, `rust-solid-slice1`, the TSX frontend branches, `project-docs` and `project-dashboard`.

- `rust-solid/` is a Rust port of Solid 2's runtime: the scheduler, async memos, stores, lists, the universal
  renderer, a plan v2 consumer, an eval table and a server renderer.
- `frontend/` holds the TSX and solid-yield frontends onto the RX AST, plan schemas v1 and v2, and the gates.

This report first compared `rxc`, the compiler crate on reactivescript `main`. That comparison is kept in a short
section near the end.

**Method.** I read the READMEs, docs, specs, source and recorded results of all three. Building or running the cloned
code was blocked in this environment. Every figure below comes from the repos' own files, not from a run of mine.

## Headline

Seen through `solid-model`, the two projects are **the same architecture built from opposite ends**. Both compile a
small checked UI language into a host-neutral **plan**: data with no source text and no app JavaScript. Both run that
plan on a JS runtime and on a Rust runtime, and render it on the server from Rust. Both treat agreement between the
two runtimes as a gate.

- **Exact starts from hosts.** It owns layout (Taffy), native AppKit, UIKit and Linux presenters, motion, storage and
  delivery. Its reactive model is its own: simple, and silent about async consistency.
- **`solid-model` starts from semantics.** "Solid" is a semantic model plus a conformance suite. Three syntaxes
  (TSX, RX, yield) lower to one plan. The Rust runtime must equal Solid JS trace for trace, including Solid 2's
  transitions, optimistic writes and held async, under thousands of fuzzed programs. It has no layout engine and no
  native window yet; slice 4, Blitz or Masonry, is planned.

So the earlier verdict, "not competitors, different slots", **no longer holds for the Rust work**. `solid-model` is
heading where Exact already is: one plan, many runtimes and hosts, server rendering from Rust, plans as data for
live code. It brings the async semantics and the conformance rigour Exact lacks. Exact is far further along in everything a
real app needs on screen: about 1.47M lines of Rust against rust-solid's 25k.

## At a glance

| | expo/exact | ReactiveScript (`main`) | solid-model (`rust-solid/` plus `frontend/`) |
|---|---|---|---|
| Goal | Write an app once and run it natively on web, macOS, iOS and Linux. Agents can build, see and test it on each. | Solid 2's reactive rules as compile-time language rules | Accept Solid-style code in any syntax only if it behaves exactly like native Solid. Runtimes, renderers and placement are plugins around that model. |
| Who | Charlie Cheever (Expo), 53 of 54 public commits (squashed history; "exact2" rebuilds "exact1") | Ryan Carniato and Claude, 252 commits since 2026-10-21 | Dev, through agents: 44 commits since 2026-10-28 (32 by the `rx-tsx-refresh` sandbox, 12 by Dev) |
| Source syntax | Contract: indentation-based, CSS-named attributes, restricted expressions | `.rx` | Solid TSX, `.rx` and solid-yield's generator syntax. All three emit **byte-identical** plans for the same program. |
| Checker | Exact's own (about 420 diagnostic ids) | RX checker (about 80 codes) | RX's checker at a pin. The TSX and yield frontends add refusals for anything that wouldn't match native Solid (async callbacks, stale write-backs, structured untrack). |
| The plan | Tables plus bytecode, generated from one JSON schema. Constants are baked in at build. One app; multi-component. | none (lowers to Solid JS) | Plan v2: JSON for **one component**: sources, memos, actions, markup, typed async `hosts`. Expressions are JSON op trees (the tracker plan is 15 KB). Built from the checked RX AST plus CP-6 facts. |
| JS runtime | `rt.js`, about 1.5k lines, a Solid-style signal graph | Solid 2 | Solid 2 itself, through `solid-plan.ts` (a plan built with Solid's own primitives) |
| Rust runtime | A plan VM with a static dependency table (bitsets, dirty-set sweep) and transactional commits with rollback, about 123k lines | — | `rust-solid`, about 25k lines. A structural port of Solid 2's scheduler, lanes, async, stores, optimistic stores, `mapArray`/For/Show and `@solidjs/universal`'s renderer. An older non-structural runtime (`lib.rs`) reached 57% and was superseded. |
| Rendering | Kernel: a node arena plus Taffy CSS layout. Hosts: DOM, AppKit, UIKit, vello/tiny-skia, early Windows and Android. | Solid's DOM, SSR and universal targets | A `Renderer` trait (create, insert, set_prop, event…) and a recording tree. **No layout and no native window yet.** |
| Server rendering | A Rust render host pre-renders pages, then adopts them in place | Solid's streamed SSR plus hydration | Rust renders a plan to HTML **byte-identical** to Solid's `renderToString`. Solid's client hydrates it in Chromium. Not yet: async server data, attributes, streaming, Rust hydration. |
| Live code | Live replacement of Contract, TS and Rust in dev. Signed plan delivery to installed apps. | — | `eval(code, bindings)` / `createEval`: code strings checked at run time and run as plans on JS or Rust, with explicit capability bindings (aimed at generative UI, REPLs and plugins) |
| Runtime conformance | `conform.mjs`: the same plan through the Rust (wasm) runner and the JS runner in Chrome, driven by real apps' agent scripts. Compares state, tree, layout boxes and screenshots. The oracle is Exact's own runner. **No fuzzing.** | Scenarios against Solid; TS reference vs Rust compiler diff | **Differential fuzzing against native Solid**, full host logs entry for entry, recorded summaries that must stay byte-identical. Seeds 1–1000: slice 1 1000/1000, async memos 991, stores 1000, lists 999, renderer 993, plans 861 agree plus 131 refused, SSR 869 agree with 0 HTML or hydration failures. Held-out ranges up to seed 8000. The tracker's 13 frames match, and its 453/453 log entries. |
| Gates | 6 blocking checks, 65 s. Per-host `app.test.contract` scripts. | five scenario runners | `verify:subset` PASS 106/106. `cargo test` 132. fmt, clippy `-D warnings`. |
| Maturity | Pre-1.0, 60+ apps, perf budgets, iOS and macOS benchmarks against UIKit and SwiftUI | 0.1, published to npm and the VS Code Marketplace | **A verified subset, not readiness**, by its own wording. Single-component, tracker-shaped programs; M1 parity is open for the general runtime. Not published. Upstream PR #21 (demo) and issues #20 and #33 have no reply yet. |

## Where they overlap

Most of this held before and still does. The new rows are about the plan.

- **A plan between language and runtime.** Exact: Contract → plan → Rust runner, or → one ES module on the web.
  solid-model: TSX, RX or yield → plan v2 → `solid-plan.ts` or `rust-solid`. Neither plan holds source text or
  app JavaScript. Both say the boot or eval path should compile nothing.
- **Two runtimes held to one behaviour.** Exact's `conform.mjs` and solid-model's plan, eval and SSR fuzz are the
  same idea. They differ in the oracle (Exact's runner vs native Solid) and in the inputs (real apps vs generated
  programs).
- **Server HTML from Rust, taken over by the client.** Exact adopts the page in place. solid-model matches Solid's
  bytes so Solid's own hydration works.
- **Plans as data, so code can change at run time.** Exact delivers signed plans to installed apps. solid-model
  evaluates checked code strings into plans.
- **Semantics.** Writes only in actions, and an action's reads see the state at its start (Exact's
  `analyze-read-after-assign`, RX's W-3 and W-4). Newest request wins. The old value stays up while a new one loads.
  Keyed lists. Optimistic writes.
- **Agent-first.** Small rules, refusals that name the fix, measured agent runs. Both use an agent-run sandbox for
  most of the work. solid-model's handoff doc is an explicit operating manual for agents, much like Exact's
  `AGENTS.md` and `rules/RULES.md`.

## Where Exact is ahead

1. **Everything on screen.** Layout, text, native controls, motion, gestures, lists, media, storage, auth and delivery
   on four hosts. rust-solid has a renderer trait and a recording tree.
2. **The plan covers a whole app.** Plan v2 is one component with integer and string values. Exact's plan carries
   components, routes, styles, fonts, timers and resources across an app.
3. **The agent loop.** Ten operations on every host, a clock the test controls, screenshots, about 99 ms from edit to
   screen, and an external agent benchmark: $3.48 per app, ahead of SvelteKit.
4. **Conformance on real apps.** Exact's two-runtime check also compares layout and pixels on real apps. solid-model
   checks traces on generated, tracker-shaped programs.

## Where solid-model and ReactiveScript are ahead

1. **Async semantics.** rust-solid ports Solid 2's lanes, transitions, held writes and optimistic stores, and the
   fuzz holds it to Solid trace for trace. Exact's native VM has newest-wins resources and atomic commits. It has no
   held, consistent frame across async work, and its docs say stale values can show after an argument change.
2. **Conformance rigour.** Seeded differential fuzzing against a reference implementation, with reduction, held-out
   seed ranges and byte-identical recorded summaries. The fuzz has also found apparent Solid rc.21 bugs, each with a
   standalone repro and none filed yet (`docs/scratch/solid-rc21-issues.md`). Exact has no fuzzing.
3. **Many syntaxes, one meaning.** TSX, RX and yield emit the same plan, and a program is refused unless it behaves
   like native Solid. Exact has one syntax with no JS or JSX tier.
4. **Static async checks.** RX refuses missing `errored` and `loading` handling and outside-region reads (unchanged
   from the first comparison).
5. **Ecosystem fit.** The JS leg *is* Solid. Exact is a closed platform.

## Ideas solid-model could borrow from Exact

In rough order of payoff:

1. **Make slice 4 a layout kernel, not just a window.** Exact's answer is a columnar node arena with patched Taffy
   behind the renderer, the host measuring text, and the browser as the oracle for layout and paint. rust-solid's
   `Renderer` trait is the right seam. Putting Taffy or Blitz's layout behind it, and checking boxes against Chrome
   the way Exact checks against Chrome, gives slice 4 a measurable gate like the others.
2. **A two-runtime conformance run on real apps, beside the fuzz.** Exact's `conform.mjs` drives real apps' test
   scripts through both runners and compares state, tree, boxes and screenshots. Adding that leg (tracker, like and
   the RX examples, through Solid JS and rust-solid) would catch what generated programs don't reach.
3. **Plans for whole apps, compactly.** Exact generates its plan format from one JSON schema and stores expressions
   as bytecode. Plan v2's JSON op trees (15 KB for the tracker) and its one-component limit will hurt once plans cover
   apps or travel over the wire for eval and placement. A schema-generated binary form, with JSON as the debug view,
   is the obvious next step.
4. **Bake the first frame.** Exact evaluates constants into the plan at build, so the first frame needs nothing else.
   A plan with seeds and pure memos could carry its first frame the same way. That pairs well with Rust SSR.
5. **An agent driver and a clock the test controls.** The scenario format already gives frames and controlled flights.
   Promoting it to Exact-style operations (tap, type, expect, settle, land or fail a flight) on every runtime makes
   scenarios usable as app tests too.
6. **Signed plan delivery.** Exact signs plans and lets installed apps fetch them after first pixel. That is the
   production counterpart of `eval`, and a natural home for the `place` and placement work.

## Ideas ReactiveScript (`main`) could borrow

These carry over from the first comparison.

1. **An overlay for optimistic state, rebased on every answer.** Exact's `overlay(source, args, answer, writes)`
   shows the newest answer with pending writes reapplied. The most common .rx eval failure fits that description: in
   round 10, 28 of 36 desks failed "labels that load while an add is out keep the add".
2. **A per-action `queue` policy** for write logs, beside newest-wins.
3. **A shipped test format with a clock the test controls** (shared with idea 5 above).
4. **A one-card vocabulary** (`rxc vocab` or `rxc explain W-1`). Spec reading is what makes .rx expensive for agents.
5. **Dev-mode shape checks** on values crossing the JS import seam.
6. **Run the external agent-framework benchmark** that Exact uses.

## The first comparison: rxc vs Exact's Contract compiler

`rxc` is reactivescript's production compiler: about 12.8k lines of Rust, from PR #6. It parses, checks and builds
the CP-6 model, then lowers through Solid's `solidjs-compiler` as an Oxc `Program` to the dom, ssr and universal
targets. It is diffed byte for byte against the TS reference. Exact's Contract compiler is about 239k lines in four
passes and owns its whole back end. That comparison still stands, but `rxc` is a compiler into Solid JS, while
`solid-model` is the work that maps onto Exact's architecture. The two Rust efforts also differ in kind: `rxc` reuses
Solid's compiler, and `rust-solid` replaces Solid's runtime.

One connection: plan v2 is built from the same CP-6 resolved model that `rxc` builds (`model.rs`). An `rxc` that
emitted plan v2 directly would put the Rust frontend and the Rust runtime on one Rust path with no TS in between.
