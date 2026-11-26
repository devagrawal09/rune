//! `Runtime::retained`: a host with its own tree writes text into committed nodes, and the next
//! commit lays them out again without running the view.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::*;
use crate::{Cx, El, Env, Span, col, row, text};

/// The host's tree: the view builds from it, and the host writes the same text through `Retained`.
struct S {
    label: Rc<RefCell<String>>,
    rich: bool,
    rows: usize,
    views: Rc<Cell<u32>>,
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    s.views.set(s.views.get() + 1);
    let label = s.label.borrow().clone();
    let len = label.len();
    let shown = if s.rich { text(label).spans(vec![Span::new(0, len).bold()]) } else { text(label) };
    // In a row the label is as wide as its text.
    let mut c = col().child(row().child(shown.key("label")));
    for i in 0..s.rows { c = c.child(text(format!("row {i}")).key(format!("row{i}"))); }
    c
}

struct Host { rt: Runtime<S>, label: Rc<RefCell<String>>, views: Rc<Cell<u32>> }

impl Host {
    fn new(rich: bool) -> Self {
        let label = Rc::new(RefCell::new("one".to_string()));
        let views = Rc::new(Cell::new(0));
        let state = S { label: label.clone(), rich, rows: 1, views: views.clone() };
        let mut rt = Runtime::new(state, view, (200.0, 200.0), Env::new(true, 1.0));
        rt.frame(0.0);
        Host { rt, label, views }
    }

    fn write(&mut self, key: &str, content: &str) {
        *self.label.borrow_mut() = content.to_string();
        self.rt.retained().set_text(key, content);
        self.rt.frame(1.0 / 60.0);
    }

    fn shown(&self, key: &str) -> Option<String> {
        self.rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).and_then(|e| e.text)
    }

    fn width(&self, key: &str) -> f32 {
        self.rt.elements().into_iter().find(|e| e.key.as_deref() == Some(key)).unwrap().rect[2]
    }
}

#[test]
fn a_text_write_shows_without_running_the_view() {
    let mut host = Host::new(false);
    let views = host.views.get();
    host.write("label", "two");
    assert_eq!(host.shown("label").as_deref(), Some("two"));
    assert_eq!(host.views.get(), views);
    assert_eq!(host.rt.retained().landed(), 1);
}

#[test]
fn a_written_text_is_laid_out_at_its_new_width() {
    let mut host = Host::new(false);
    let before = host.width("label");
    host.write("label", "a much longer label than before");
    assert!(host.width("label") > before);
}

#[test]
fn a_write_to_a_key_that_isnt_mounted_is_dropped() {
    let mut host = Host::new(false);
    host.rt.retained().set_text("nowhere", "lost");
    host.rt.frame(1.0 / 60.0);
    assert_eq!(host.shown("label").as_deref(), Some("one"));
    assert_eq!(host.rt.retained().landed(), 1);
}

#[test]
fn a_structural_change_runs_the_view() {
    let mut host = Host::new(false);
    let views = host.views.get();
    host.rt.state.rows = 2;
    host.rt.retained().structure_changed();
    host.rt.frame(1.0 / 60.0);
    assert_eq!(host.views.get(), views + 1);
    assert_eq!(host.shown("row1").as_deref(), Some("row 1"));
}

#[test]
fn rich_text_falls_back_to_the_view() {
    let mut host = Host::new(true);
    let views = host.views.get();
    host.write("label", "two");
    assert_eq!(host.views.get(), views + 1);
    assert_eq!(host.rt.retained().landed(), 0);
}
