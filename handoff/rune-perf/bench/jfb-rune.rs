// jfb keyed on plain Rune: app state is a Vec of rows, the view is a function of
// it, each row keyed by id. Headless Runtime, 400 x 800, no paint. Timed region:
// the change as a handler makes it (Runtime::update), then one frame (view,
// keyed diff, Taffy layout, text shaping).
//   RUNE_VIRTUAL  the rows go in a virtual_rows list (Rune's idiom for long
//                 lists): only the rows in the mounted window are built and laid out.
//   RUNE_MEMO     each row is memo(id, (label, selected)).
// Run from the rune fork:
//   cargo run --release -p rune --no-default-features --features embedded-fonts --example jfb_rune -- [op|all] [reps]
#[path = "jfb_common.rs"]
mod common;
use std::borrow::Cow;

use common::{Driver, Row, Shown};
use rune::{Cx, El, Env, GrowthSpec, Runtime, ScrollSpec, col, el, memo, row, text};

#[derive(Default)]
struct S { rows: Vec<Row>, next: u64, sel: Option<u64>, ver: u64, memo: bool }

fn line(r: &Row, sel: bool) -> El {
    let id = r.id;
    row().key(format!("r{id}"))
        .child(text(if sel { "danger" } else { "-" }))
        .child(text(id.to_string()))
        .child(el().on_press(move |s: &mut S| s.sel = Some(id)).child(text(r.label.clone())))
        .child(el().on_press(move |s: &mut S| s.rows.retain(|r| r.id != id)).child(el().size(8.0, 8.0)))
        .child(el())
}
fn build(s: &S, i: usize) -> El {
    let r = &s.rows[i];
    let sel = s.sel == Some(r.id);
    if s.memo { let r2 = r.clone(); memo(format!("m{}", r.id), (r.label.clone(), sel), move || line(&r2, sel)) } else { line(r, sel) }
}

fn view(s: &S, _: &mut Cx<S>) -> El {
    (0..s.rows.len()).fold(col().key("tbody"), |c, i| c.child(build(s, i)))
}
fn view_virtual(s: &S, _: &mut Cx<S>) -> El {
    col().grow(1.0).child(col().scroll(ScrollSpec { stick_bottom: false }).grow(1.0).child(
        col().key("tbody").growth(GrowthSpec::default())
            .virtual_rows(s.rows.len(), 20.0, |s: &S, i| Cow::Owned(format!("r{}", s.rows[i].id)), build)
            .virtual_rows_version(s.ver)))
}

/// The text Rune lays out, grouped into rows of three cells, top to bottom.
pub fn shown_rows<St: 'static>(rt: &Runtime<St>) -> Vec<(String, String, String)> {
    let mut v: Vec<_> = rt.elements().into_iter()
        .filter(|e| !e.exiting)
        .filter_map(|e| Some((e.target[1], e.target[0], e.text?)))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    v.chunks(3).map(|c| {
        assert!(c.len() == 3 && c[0].0 == c[2].0, "cells out of line: {c:?}");
        (c[0].2.clone(), c[1].2.clone(), c[2].2.clone())
    }).collect()
}

struct Rn { rt: Runtime<S>, virt: bool }
impl Rn {
    fn act(&mut self, f: impl FnOnce(&mut S)) {
        self.rt.update(|s, _| { f(s); s.ver += 1 });
        self.rt.frame(1.0 / 120.0);
    }
}
impl Driver for Rn {
    fn run(&mut self) { self.act(|s| s.rows = common::build(&mut s.next, 1000)) }
    fn runlots(&mut self) { self.act(|s| s.rows = common::build(&mut s.next, 10_000)) }
    fn add(&mut self) { self.act(|s| { let more = common::build(&mut s.next, 1000); s.rows.extend(more) }) }
    fn update(&mut self) { self.act(|s| for i in (0..s.rows.len()).step_by(10) { s.rows[i].label += " !!!" }) }
    fn clear(&mut self) { self.act(|s| s.rows.clear()) }
    fn swap(&mut self) { self.act(|s| if s.rows.len() > 998 { s.rows.swap(1, 998) }) }
    // The rows' own press handlers, as `line` attaches them.
    fn select(&mut self, i: usize) { self.act(|s| s.sel = Some(s.rows[i].id)) }
    fn remove(&mut self, i: usize) { self.act(|s| { let id = s.rows[i].id; s.rows.retain(|r| r.id != id) }) }
    fn shown(&mut self) -> Shown {
        for _ in 0..240 { if !self.rt.frame(1.0 / 120.0) { break; } }
        Shown { rows: shown_rows(&self.rt), windowed: self.virt }
    }
}

fn main() {
    let virt = std::env::var("RUNE_VIRTUAL").is_ok();
    let m = std::env::var("RUNE_MEMO").is_ok();
    let name = match (virt, m) { (false, false) => "rune", (true, false) => "rune-virtual", (false, true) => "rune-memo", (true, true) => "rune-virtual-memo" };
    common::main_with(name, || {
        let mut rt = Runtime::new(S { next: 1, memo: m, ..S::default() }, if virt { view_virtual } else { view }, (400.0, 800.0), Env::new(true, 1.0));
        for _ in 0..240 { if !rt.frame(1.0 / 120.0) { break; } }
        Rn { rt, virt }
    });
}
