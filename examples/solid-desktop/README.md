# Solid desktop apps on Rune

Four small desktop apps written as Solid 2 components in TSX. Each one is compiled to a
*plan* (a JSON description of its signals, memos, actions and markup), run by
[rust-solid](https://github.com/devagrawal09/reactivescript/tree/desktop-demos/rust-solid)
(a Rust port of Solid 2's reactive core), and drawn by Rune. There is no JavaScript engine
and no web view: the signal graph runs in Rust, in the same thread as the window.

| App | Shows |
| --- | --- |
| `todos` | A text field, filters, toggles and removals that show at once and roll back when the save fails (every fourth save fails). |
| `issues` | A two-pane tracker: search that keeps only the latest answer, an async detail pane that holds the previous issue until the next one loads, a comment box, an optimistic close, and an issue (#104) whose load fails with a retry. |
| `chat` | A conversation: your message shows as you send it, Ada's typing line comes and goes, the third send fails and offers the text back. |
| `weather` | City tabs over a slow forecast service, a °C/°F switch, a refresh that dims the old forecast until the new one lands, and a city (Reykjavik) whose first load fails. |

Every app has an activity strip at the bottom: the last five host calls (what the app asked
its backend, how long it took and what came back) and three counters, `host calls`,
`view runs` (how often Rune rebuilt the view) and `text-only commits` (how often a change
was only text and went straight into the drawn elements without a view run).

## Setup

The example depends on rust-solid by path, so clone the two repos side by side:

```sh
git clone -b solid-desktop-demos https://github.com/devagrawal09/rune
git clone -b desktop-demos https://github.com/devagrawal09/reactivescript
```

The layout has to be:

```
rune/
reactivescript/
```

`examples/solid-desktop/Cargo.toml` points at `../../../reactivescript/rust-solid/crates/runtime`.
Because the example is a workspace member, any Cargo command in this checkout needs that
clone to be there.

## Run a window

```sh
cd rune
cargo run --release -p solid-desktop --bin todos
cargo run --release -p solid-desktop --bin issues
cargo run --release -p solid-desktop --bin chat
cargo run --release -p solid-desktop --bin weather
```

On a Mac this opens a native window on Metal. Things to try:

- **Todos:** type in the field and press Enter, tick and untick rows, remove rows, switch the
  tabs. The fourth save fails: the row jumps back and the footer says why.
- **Issues:** click through the list, search for `dark`, post a comment, close and reopen an
  issue, and open #104 to see the failure and Retry.
- **Chat:** send a few messages. The third one fails; press Edit to put it back in the field.
- **Weather:** switch cities quickly (only the last one you picked lands), flip °C/°F, press
  Refresh, and open Reykjavik twice.

## Headless

Rune can drive a window on a virtual clock with no display. The apps here build with Rune's
`tooling` and `keys-overlay` features, so the release binaries take these flags:

```sh
# A still at 3.5 s, after the scripted taps and typing up to then
./target/release/chat --script examples/solid-desktop/scripts/chat.json --snap chat.png --at 3.5

# A video of the whole script, with the pointer and keys drawn
./target/release/chat --script examples/solid-desktop/scripts/chat.json --video chat.mp4 --fps 30 --pointer --keys

# A still of the first frame with no script
./target/release/weather --snap weather.png --at 2
```

The scripts in `scripts/` tap elements by key (`id:send` is the element whose `id` attribute
is `send`), type text, press keys, and end at the `end` mark. The video runs to that mark.

## Change an app

The TSX source is in `apps/<name>/<name>.tsx`, with the backend's signatures in
`apps/<name>/api.d.rx`. The compiled plans in `plans/` are checked in. To rebuild one after
an edit, run the emitter from the reactivescript frontend:

```sh
cd ../reactivescript/frontend
npm install
node --experimental-strip-types --conditions=browser \
  ../../rune/examples/solid-desktop/apps/emit.ts \
  ../../rune/examples/solid-desktop/apps/chat/chat.tsx \
  ../../rune/examples/solid-desktop/plans/chat.plan.json
```

The compiler checks the component the way RX does: an async read with no `Loading` or
`Errored` above it, an action that can fail and does not catch, or a read of a signal the
same action just wrote are compile errors.

The backend for each app is the `Backend` impl in `src/bin/<name>.rs`: it answers each host
call with a result and a delay on the window's clock.

## How it fits together

- `src/host.rs` mounts the plan, issues the host calls it makes, settles them when their
  delay is up, and reads back what the Solid tree changed since the last frame.
- A change to the tree's shape or to an element's attributes asks Rune to run the view. A
  change that is only text is written into the already-drawn element through
  `Cx::retained()`, with no view run.
- `src/view.rs` turns the Solid tree into Rune elements. Every node keeps one key, so a row
  that Solid rebuilds with the same `id` keeps its place and animates instead of popping.
- `src/look.rs` is the stylesheet: what each tag and class looks like in Rune.

## What headless runs leave out

The stills and videos come from the virtual clock, so they don't show GPU presentation
timing, 120 Hz scrolling and springs on a real display, native text input (IME, dictation,
the macOS text services), hover under a real pointer, or window resizing. Run the apps in a
window for those.
