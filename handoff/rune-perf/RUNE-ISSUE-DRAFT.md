# Draft issue for anomalyco/rune (not posted)

Title: A retained write path for hosts that already know what changed

---

Hi, and thanks for Rune. The headless `Runtime`, `virtual_rows` and `Tx::live_only` made this
experiment easy to run.

**Context.** We're porting Solid 2's reactive runtime to Rust and tried Rune as its renderer.
A fine-grained runtime already knows exactly which text node changed and which child list
gained or lost a node. Through `El`, the only seam today, Rune can't use that: every change
re-runs the view, flattens and diffs, even when one string changed.

**What we tried.** Five small workloads, headless, 400 x 800 window, µs per click, best of five,
one machine and one session:

| | Rune, plain view | Rune, `virtual_rows` | Glue, rebuild | Glue, windowed | Retained + windowed |
|---|---:|---:|---:|---:|---:|
| counter (1 text) | 28 | – | 35 | 34 | 33 |
| fanout (1,000 texts change) | 1,714 | 495 | 3,290 | 1,221 | 1,336 |
| select (2 of 1,000 rows change) | 1,197 | 235 | 2,087 | 235 | 128 |
| filter (±500 rows) | 1,591 | 688 | 4,842 | 2,223 | 1,831 |

- *Glue* builds an `El` tree from the Solid host's tree each frame.
- *Windowed* adds `virtual_rows` plus `on_visible_rows`, and Solid creates only the rows Rune
  shows.
- *Retained* is a local patch to Rune, about 125 lines. A `Retained` handle queues
  `set_text(key, text)` writes. The handler calls `Tx::live_only`, and the next commit writes
  the text into the committed nodes, dirties their layout and reuses the resize path. It skips
  the view, flatten and diff. Structural changes still go through the view.
- After every click, every variant shows the same text as the plain Rune view.

**What we found.**

- Text updates the host already knows about get cheap. Select drops to 128 µs, below plain
  Rune with `virtual_rows`, because only the two changed strings are written and re-laid out.
- The rest is mostly layout. When one leaf of a 1,000-child flex column changes, Taffy reruns
  flexbox over every child, about half of a full-list select click. A fast path for columns of
  fixed-height children, or paint-only props that skip layout, would help retained writes most.
- Structural changes (filter) still run the view, so insert and remove are the next piece.
- Two traps that a public API should make hard to hit:
  - The write path must release the old text layout as the diff does. Without it, the layout
    cache grew with every new string and a counter click went from 49 to 84 µs over 50,000
    clicks.
  - `virtual_rows` replaces a row's key with the list's key and keeps the key list per version.
    A host writing by its own keys can then target a key that isn't mounted, and the write is
    silently dropped. A debug warning would catch it.

**What we'd ask for.** A small public retained API on the committed tree, with the view kept as
the source of truth and the fallback:

```rust
let ret = rt.retained();                     // cloneable, usable inside handlers
ret.set_text(id, "row 7");                   // plain text; dirties that leaf
ret.set_style(id, |s| s.fill(color));        // paint-only props skip layout
ret.insert(parent, before, el);              // flatten one subtree in place
ret.remove(id);                              // exits and prunes as today
ret.move_before(id, parent, before);
// handler: tx.live_only(); the next commit applies only these writes
```

- Stable handles (`NodeId` for a keyed element) so a host doesn't look keys up per write.
- `insert`/`remove`/`move` splice the node arena and re-sync only the changed parent's
  `LayoutStore` child slice, with presence and exit handled as a keyed diff would.
- Debug builds could check retained writes the way they check `Tx::live_only`: run the view
  now and then and warn when it disagrees, or when a write names a key that isn't mounted.

We're not asking Rune to adopt signals. This only lets a host that already tracks changes skip
work Rune would repeat. We're happy to share the prototype patch and the benchmark if that
helps.
