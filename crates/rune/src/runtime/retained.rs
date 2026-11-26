//! A retained write path beside the view, for hosts that keep their own tree (a fine-grained
//! reactive runtime). Text written through [`Retained`] goes straight into the committed nodes,
//! and the next commit lays out only what it dirtied, as a resize does: no view, no flatten,
//! no diff. A write to a key that isn't mounted is dropped, so the view must build what the host
//! shows when it mounts it. A structural change or rich text runs the view.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;
use super::commit::text_layout::edited;

#[derive(Default)]
pub(in crate::runtime) struct Edits {
    texts: Vec<(String, String)>,
    structure: bool,
    landed: u64,
}

/// A handle to a runtime's retained writes. Clone it into a handler: writes made there commit
/// in the same frame. Pair them with [`Tx::live_only`] so the handler doesn't also run the view.
#[derive(Clone, Default)]
pub struct Retained(Rc<RefCell<Edits>>);

impl Retained {
    /// Sets the text of the committed element keyed `key`.
    pub fn set_text(&self, key: impl Into<String>, text: impl Into<String>) {
        self.0.borrow_mut().texts.push((key.into(), text.into()));
    }

    /// Elements were inserted, removed or moved: the next commit runs the view.
    pub fn structure_changed(&self) {
        self.0.borrow_mut().structure = true;
    }

    /// How many commits wrote retained text in place, skipping the view.
    pub fn landed(&self) -> u64 {
        self.0.borrow().landed
    }

    pub(in crate::runtime) fn pending(&self) -> bool {
        let e = self.0.borrow();
        e.structure || !e.texts.is_empty()
    }

    pub(in crate::runtime) fn structural(&self) -> bool {
        self.0.borrow().structure
    }

    pub(in crate::runtime) fn take(&self) -> Edits {
        let mut e = self.0.borrow_mut();
        let landed = e.landed;
        let out = std::mem::take(&mut *e);
        e.landed = landed;
        out
    }
}

/// Plain text only: anything shaped from more than the content goes through the view.
fn writable(n: &Node) -> bool {
    n.input.is_none()
        && n.text.as_ref().is_some_and(|t| {
            t.spans.is_empty() && t.cold.is_empty() && !t.redacted && t.case.is_none() && t.source.is_none()
        })
}

impl<S: 'static> Runtime<S> {
    /// The handle a host writes through. See [`Retained`].
    pub fn retained(&self) -> Retained {
        self.core().commit.retained.clone()
    }
}

impl RuntimeCore {
    /// Writes the pending text into the committed nodes and dirties their layout. False when
    /// any write can't land in place; nothing is written then and the commit runs the view.
    pub(in crate::runtime) fn write_retained(&mut self) -> bool {
        let edits = self.commit.retained.take();
        if edits.structure {
            return false;
        }
        let nodes = &self.tree.nodes;
        let index = (edits.texts.len() > key_index::SCAN_READS).then(|| key_index::KeyIndex::of(nodes));
        let mut at = Vec::with_capacity(edits.texts.len());
        for (key, content) in edits.texts {
            let i = match &index {
                Some(ix) => ix.first(nodes, &key),
                None => key_index::scan_first(nodes, &key),
            };
            match i {
                // Not mounted (a culled virtual row): the view builds it from the host when shown.
                None => continue,
                Some(i) if writable(&nodes[i]) && !self.subtree_exiting(i) => at.push((i, content)),
                Some(_) => return false,
            }
        }
        let scale = self.env.scale;
        for (i, content) in at {
            let t = self.tree.nodes[i].text.as_mut().expect("writable nodes have text");
            if t.content == content {
                continue;
            }
            if let Some((_, shape)) = t.shape && edited(&t.content, &content) {
                self.text.supersede(shape);
            }
            t.content = content;
            t.key_shape(scale);
            // Paint copies a subtree whose nodes keep their epochs; this one changed.
            self.tree.nodes[i].epoch = paint_keep::next_epoch();
            let nodes = &self.tree.nodes;
            self.commit.layout.mark_dirty_and_ancestors(
                nodes[i].taffy,
                std::iter::successors(nodes[i].parent, |&p| nodes[p].parent).map(|p| nodes[p].taffy),
            );
        }
        self.commit.retained.0.borrow_mut().landed += 1;
        true
    }
}
