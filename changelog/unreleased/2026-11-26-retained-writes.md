- **A host that keeps its own tree can write text without running the view.** `Cx::retained`
  (and `Runtime::retained`) hands out a `Retained` handle: `set_text(key, text)` writes into the
  committed element with that key, and the next commit lays out only what changed, as a resize
  does, with no view, flatten or diff. A write to a key that isn't mounted is dropped; rich or
  styled text, or `structure_changed()`, runs the view as before. Built for fine-grained reactive
  runtimes (a Solid-style signal graph) driving a Rune window.
