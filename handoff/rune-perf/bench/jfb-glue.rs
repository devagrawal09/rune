// jfb keyed through the Rune glue: rust-solid owns state and the tree (the
// same plan and store writes as the rust-solid bench), Rune shows it. Retained
// mode is on: rust-solid's text writes go into Rune's committed nodes through
// Runtime::retained (the local Rune patch) with no view run; a changed child
// list runs the view, which rebuilds an El from rust-solid's tree.
//   GLUE_LAZY  the rows are a windowed Solid program (<For each={list.slice(
//              winStart, winEnd)}> in a "vlist"), shown as a Rune virtual list
//              whose on_visible_rows sets the window, so rust-solid creates
//              only the rows Rune shows plus one viewport either side.
// Timed region: the write or click in rust-solid, one turn, then one Rune frame.
// Run from the rune fork:
//   cargo run --profile perf -p rune --no-default-features --features embedded-fonts --example jfb_glue -- [op|all] [reps]
#[path = "jfb_common.rs"]
mod common;
#[path = "jfb_solid_plan.rs"]
mod solid;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use common::{Driver, Shown};
use rune::{Cx, El, Env, GrowthSpec, Retained, Runtime, ScrollSpec, Tx, col, el, row, text};
use rust_solid::plan_v2::App;
use rust_solid::renderer::{HostWrite, TreeRenderer};
use rust_solid::slice1::Value;
use solid::Ops;

struct G {
    app: Rc<RefCell<App>>,
    lazy: bool,
    count: Cell<usize>,
    win: Cell<(usize, usize)>,
    kids: RefCell<(usize, u64, Vec<usize>)>,
    ret: Retained,
    moves: Cell<u64>,
}

impl G {
    /// rust-solid's host writes, repeated into Rune: text in place, anything else by the view.
    fn flush(&self) {
        let app = self.app.borrow();
        let mut h = app.host.borrow_mut();
        for w in h.take_writes() {
            match w {
                HostWrite::Text(n) => self.ret.set_text(format!("t{n}"), h.glue_node(n).1.to_js_string().unwrap_or_default()),
                HostWrite::Children(_) => self.ret.structure_changed(),
            }
        }
    }
    fn refresh(&self) {
        if !self.lazy { return; }
        if let Ok(Value::Int(n)) = self.app.borrow_mut().value("count") {
            // The list's length is the virtual list's row count: the view must run.
            if self.count.replace(n as usize) != n as usize { self.ret.structure_changed(); }
        }
    }
    /// After rust-solid ran: the window's length, then its writes.
    fn after(&self) { self.app.borrow_mut().turn(); self.refresh(); self.flush(); }
    /// Child `j` of `n`, from a cache refreshed when `n`'s child list changes.
    fn kid(&self, h: &TreeRenderer, n: usize, j: usize) -> Option<usize> {
        let kv = h.glue_kver(n);
        let stale = { let k = self.kids.borrow(); k.0 != n || k.1 != kv };
        if stale { *self.kids.borrow_mut() = (n, kv, h.glue_kids(n).collect()); }
        self.kids.borrow().2.get(j).copied()
    }
}

/// Each host text is a Rune text keyed by its node, which is where retained
/// writes land; each element a container keyed by its node, a "tr" laid out as a row.
fn build(s: &G, h: &TreeRenderer, n: usize) -> El {
    let (tag, txt, _) = h.glue_node(n);
    let Some(tag) = tag else { return text(txt.to_js_string().unwrap_or_default()).key(format!("t{n}")) };
    if tag == "vlist" { return vlist(s, h, n); }
    let c = if tag == "tr" { row() } else { col() }.key(format!("k{n}"));
    let c = if h.handles(n, "click") {
        c.on_press_tx(move |s: &mut G, tx: &mut Tx| { s.app.borrow_mut().click(n).unwrap(); s.after(); tx.live_only() })
    } else if tag == "span" { c.size(8.0, 8.0) } else { c };
    h.glue_kids(n).fold(c, |c, k| c.child(build(s, h, k)))
}

fn vlist(s: &G, h: &TreeRenderer, n: usize) -> El {
    let rows = col().key(format!("v{n}")).growth(GrowthSpec::default())
        .virtual_rows(s.count.get(), 20.0, move |s: &G, i| {
            let app = s.app.borrow();
            let h = app.host.borrow();
            // A row's key is its root's, the one `build` gives it.
            match i.checked_sub(s.win.get().0).and_then(|j| s.kid(&h, n, j)) {
                Some(k) => Cow::Owned(format!("k{k}")),
                None => Cow::Owned(format!("i{i}")),
            }
        }, move |s: &G, i| {
            let app = s.app.borrow();
            let h = app.host.borrow();
            match i.checked_sub(s.win.get().0).and_then(|j| s.kid(&h, n, j)) { Some(k) => build(s, &h, k), None => el().size(1.0, 20.0) }
        })
        .virtual_rows_version((s.win.get(), h.glue_kver(n)))
        .on_visible_rows(|s: &mut G, r| {
            let over = r.len().max(1);
            let (a, b) = s.win.get();
            if a <= r.start && r.end.min(s.count.get()) <= b { return false; }
            let want = (r.start.saturating_sub(over), r.end + over);
            s.win.set(want);
            s.moves.set(s.moves.get() + 1);
            {
                let mut app = s.app.borrow_mut();
                app.set_signal("winStart", Value::Int(want.0 as i64)).unwrap();
                app.set_signal("winEnd", Value::Int(want.1 as i64)).unwrap();
            }
            s.after();
            true
        });
    col().key(format!("k{n}")).grow(1.0)
        .child(col().key(format!("s{n}")).scroll(ScrollSpec { stick_bottom: false }).grow(1.0).child(rows))
}

fn view(s: &G, _: &mut Cx<G>) -> El {
    let app = s.app.borrow();
    let h = app.host.borrow();
    build(s, &h, h.root())
}

/// The text Rune lays out, grouped into rows of three cells, top to bottom
/// (as in jfb_rune.rs).
fn shown_rows<St: 'static>(rt: &Runtime<St>) -> Vec<(String, String, String)> {
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

struct Glue { rt: Runtime<G>, app: Rc<RefCell<App>>, ops: Ops, body: usize, lazy: bool }

impl Glue {
    /// One change in rust-solid, then one Rune frame. Like a handler that
    /// called `Tx::live_only`: the view runs only if the child lists changed.
    fn act(&mut self, f: impl FnOnce(&mut App, &mut Ops)) {
        f(&mut self.app.borrow_mut(), &mut self.ops);
        self.rt.update_if(|g, _| { g.after(); false });
        // The window follows a change a frame late: on_visible_rows runs in
        // the frame after the commit that changed the list, and the rows it
        // makes rust-solid create commit in the frame after that. Time frames
        // until two in a row leave the window where it is.
        let (mut quiet, mut frames) = (0, 0);
        while quiet < if self.lazy { 2 } else { 1 } && frames < 16 {
            let moved = self.rt.state.moves.get();
            self.rt.frame(1.0 / 120.0);
            frames += 1;
            quiet = if self.rt.state.moves.get() == moved { quiet + 1 } else { 0 };
        }
    }
    fn press(&mut self, row: usize, cell: usize) {
        let a = solid::link(&self.app.borrow(), self.body, row, cell);
        self.act(|app, _| app.click(a).unwrap());
    }
}

impl Driver for Glue {
    fn run(&mut self) { self.act(|a, o| o.set_rows(a, 1000)) }
    fn runlots(&mut self) { self.act(|a, o| o.set_rows(a, 10_000)) }
    fn add(&mut self) { self.act(|a, o| o.add(a)) }
    fn update(&mut self) { self.act(|a, o| o.update(a)) }
    fn clear(&mut self) { self.act(|a, o| o.clear(a)) }
    fn swap(&mut self) { self.act(|a, o| o.swap(a)) }
    fn select(&mut self, row: usize) { self.press(row, 2) }
    fn remove(&mut self, row: usize) { self.press(row, 3) }
    fn shown(&mut self) -> Shown {
        for _ in 0..240 { if !self.rt.frame(1.0 / 120.0) { break; } }
        self.app.borrow_mut().check().unwrap();
        // What Rune shows must be what rust-solid holds, and (checked by the
        // caller) what the table should be.
        let rows = shown_rows(&self.rt);
        let held = solid::table(&self.app.borrow(), self.body);
        // A window: Rune mounts fewer rows than rust-solid holds.
        let n = if self.lazy { rows.len().min(held.len()) } else { held.len() };
        assert_eq!(rows[..], held[..n], "Rune shows other rows than rust-solid holds");
        if self.lazy {
            let w = self.rt.state.win.get();
            assert_eq!(w.0, 0, "window starts at the top");
        }
        Shown { rows, windowed: self.lazy }
    }
}

impl Drop for Glue { fn drop(&mut self) { self.app.borrow_mut().dispose() } }

fn main() {
    let lazy = std::env::var("GLUE_LAZY").is_ok();
    common::main_with(if lazy { "glue-retained-lazy" } else { "glue-retained" }, || {
        let mut app = App::mount_unlogged(if lazy { solid::plan_lazy() } else { solid::plan() }).unwrap();
        app.turn();
        let body = solid::body(&app);
        let app = Rc::new(RefCell::new(app));
        let g = G { app: app.clone(), lazy, count: Cell::new(0), win: Cell::new((0, 0)), kids: RefCell::new((usize::MAX, 0, vec![])), ret: Retained::default(), moves: Cell::new(0) };
        let mut rt = Runtime::new(g, view, (400.0, 800.0), Env::new(true, 1.0));
        rt.state.ret = rt.retained();
        app.borrow().host.borrow_mut().record_writes(true);
        for _ in 0..240 { if !rt.frame(1.0 / 120.0) { break; } }
        Glue { rt, app, ops: Ops { next: 1 }, body, lazy }
    });
}
