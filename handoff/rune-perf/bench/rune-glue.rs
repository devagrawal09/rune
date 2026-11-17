// Glue prototype (local, not pushed): rust-solid's plan_v2 App owns state and
// its retained TreeRenderer tree; Rune's view rebuilds an El from that tree
// each frame.
//   GLUE_MEMO     each element is wrapped in memo(n{id}, ver), ver being the
//                 stamp TreeRenderer bumps on a node and its ancestors when
//                 rust-solid writes under it.
//   GLUE_VIRTUAL  an element with more than 64 children becomes a Rune
//                 virtual list, so Rune builds and lays out only its window.
//   GLUE_LAZY     the list plans become windowed Solid programs:
//                 <For each={list().slice(winStart(), winEnd())}> inside a
//                 "vlist" element, with a `count` memo. Rune's on_visible_rows
//                 sets the window signals, as a scroll handler would, so
//                 rust-solid only creates the rows Rune shows (plus one
//                 viewport either side). Implies GLUE_VIRTUAL for other lists.
//   GLUE_RETAINED rust-solid's text writes go straight into Rune's committed
//                 nodes through Runtime::retained (a local Rune change): a
//                 click that only changes text runs no view, flatten or diff,
//                 and lays out only what the writes dirtied. A changed child
//                 list still rebuilds through the view. Combines with the rest.
//   GLUE_CHECK    instead of timing, compare what Rune shows after each of 60
//                 clicks with plain Rune's views for the same workload.
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use rune::{Cx, El, Env, GrowthSpec, Retained, Runtime, ScrollSpec, Tx, col, el, memo, text};
use rust_solid::plan_v2::{App, Plan};
use rust_solid::renderer::{HostWrite, TreeRenderer};
use rust_solid::slice1::Value;
use serde_json::{Value as J, json};

const ROWS: usize = 1000;
const FAN: usize = 1000;

fn size() -> (f32, f32) { if std::env::var("TALL").is_ok() { (400.0, 100_000.0) } else { (400.0, 800.0) } }
fn flag(n: &str) -> bool { std::env::var(n).is_ok() }

struct G {
    app: Rc<RefCell<App>>,
    b: usize,
    memo: bool,
    virt: bool,
    seen: Rc<Cell<i64>>,
    // Lazy lists: the full length (the plan's `count` memo), the window the
    // plan materialises, and the vlist's children by kver.
    has_count: bool,
    count: Cell<usize>,
    win: Cell<(usize, usize)>,
    kids: RefCell<(usize, u64, Vec<usize>)>,
    retained: bool,
    ret: RefCell<Option<Retained>>,
}

impl G {
    /// Repeats rust-solid's writes into Rune: text in place, anything else by the view.
    fn flush(&self) {
        let ret = self.ret.borrow();
        let Some(ret) = ret.as_ref() else { return };
        let app = self.app.borrow();
        let mut h = app.host.borrow_mut();
        for w in h.take_writes() {
            match w {
                HostWrite::Text(n) => ret.set_text(format!("t{n}"), h.glue_node(n).1.to_js_string().unwrap_or_default()),
                HostWrite::Children(_) => ret.structure_changed(),
            }
        }
    }

    fn refresh(&self) {
        if !self.has_count { return; }
        let mut app = self.app.borrow_mut();
        if let Ok(Value::Int(n)) = app.value("count") { self.count.set(n as usize); }
    }
}

impl G {
    /// Child `j` of `n`, from a cache refreshed when `n`'s child list changes
    /// (the host keeps children as a linked list).
    fn kid(&self, h: &TreeRenderer, n: usize, j: usize) -> Option<usize> {
        let kv = h.glue_kver(n);
        let stale = { let k = self.kids.borrow(); k.0 != n || k.1 != kv };
        if stale { *self.kids.borrow_mut() = (n, kv, h.glue_kids(n).collect()); }
        self.kids.borrow().2.get(j).copied()
    }
}

fn int(v: Value) -> i64 { match v { Value::Int(n) => n, other => panic!("not an int: {other:?}") } }

/// A virtual list's row. Rune keys a row by its list's key for it, so a retained
/// host wraps it to keep the text's own key.
/// A virtual row's key. Rune puts it on the row's root, so in retained mode it must be the key
/// `build` gives that root, the one retained writes look up.
fn row_key(s: &G, h: &TreeRenderer, n: usize, otherwise: String) -> Cow<'static, str> {
    if !s.retained {
        return Cow::Owned(otherwise);
    }
    if n == s.b {
        return Cow::Borrowed("b");
    }
    let kids: Vec<usize> = h.glue_kids(n).collect();
    Cow::Owned(match h.glue_node(n).0 {
        None => format!("t{n}"),
        Some(_) if kids.len() == 1 && h.glue_node(kids[0]).0.is_none() => format!("t{}", kids[0]),
        Some(_) => format!("k{n}"),
    })
}

fn build(s: &G, h: &TreeRenderer, n: usize) -> El {
    let (tag, txt, ver) = h.glue_node(n);
    let Some(tag) = tag else {
        let t = text(txt.to_js_string().unwrap_or_default());
        return if s.retained { t.key(format!("t{n}")) } else { t };
    };
    if n == s.b && s.retained {
        return el().key("b").size(40.0, 20.0).on_press_tx(|s: &mut G, tx: &mut Tx| {
            s.seen.set(s.seen.get() + 1);
            {
                let mut app = s.app.borrow_mut();
                app.click(s.b).unwrap();
                app.turn();
            }
            s.refresh();
            s.flush();
            tx.live_only();
        });
    }
    if n == s.b {
        return el().key("b").size(40.0, 20.0).on_press(|s: &mut G| {
            s.seen.set(s.seen.get() + 1);
            {
                let mut app = s.app.borrow_mut();
                app.click(s.b).unwrap();
                app.turn();
            }
            s.refresh();
        });
    }
    if tag == "vlist" {
        let rows = col().key(format!("v{n}")).growth(GrowthSpec::default())
            .virtual_rows(s.count.get(), 20.0, move |s: &G, i| {
                let app = s.app.borrow();
                let h = app.host.borrow();
                match i.checked_sub(s.win.get().0).and_then(|j| s.kid(&h, n, j)) {
                    Some(k) => row_key(s, &h, k, format!("i{i}")),
                    None => Cow::Owned(format!("i{i}")),
                }
            }, move |s: &G, i| {
                let app = s.app.borrow();
                let h = app.host.borrow();
                let child = i.checked_sub(s.win.get().0).and_then(|j| s.kid(&h, n, j));
                match child { Some(k) => build(s, &h, k), None => el().size(1.0, 20.0) }
            })
            .virtual_rows_version(if s.retained { (s.win.get(), h.glue_kver(n)) } else { ((0, 0), 0) })
            .on_visible_rows(|s: &mut G, r| {
                let over = r.len().max(1);
                let (a, b) = s.win.get();
                if a <= r.start && r.end.min(s.count.get()) <= b { return false; }
                let want = (r.start.saturating_sub(over), r.end + over);
                s.win.set(want);
                {
                    let mut app = s.app.borrow_mut();
                    app.set_signal("winStart", Value::Int(want.0 as i64)).unwrap();
                    app.set_signal("winEnd", Value::Int(want.1 as i64)).unwrap();
                    app.turn();
                }
                s.refresh();
                s.flush();
                true
            });
        return col().key(format!("k{n}")).grow(1.0)
            .child(col().key(format!("s{n}")).scroll(ScrollSpec { stick_bottom: false }).grow(1.0).child(rows));
    }
    let kids: Vec<usize> = h.glue_kids(n).collect();
    if s.virt && kids.len() > 64 {
        let rows = col().key(format!("v{n}")).growth(GrowthSpec::default()).virtual_rows(
            kids.len(),
            20.0,
            move |s: &G, i| {
                let app = s.app.borrow();
                let h = app.host.borrow();
                let id = s.kid(&h, n, i).unwrap();
                row_key(s, &h, id, format!("k{id}"))
            },
            move |s: &G, i| {
                let app = s.app.borrow();
                let h = app.host.borrow();
                build(s, &h, s.kid(&h, n, i).unwrap())
            },
        ).virtual_rows_version(h.glue_kver(n));
        return col().key(format!("k{n}")).grow(1.0)
            .child(col().key(format!("s{n}")).scroll(ScrollSpec { stick_bottom: false }).grow(1.0).child(rows));
    }
    let make = || {
        if kids.len() == 1 && h.glue_node(kids[0]).0.is_none() {
            // Retained writes find the text by its own node's key.
            let key = if s.retained { format!("t{}", kids[0]) } else { format!("k{n}") };
            return text(h.glue_node(kids[0]).1.to_js_string().unwrap_or_default()).key(key);
        }
        let c = col().key(format!("k{n}"));
        let c = if s.virt { c.grow(1.0) } else { c };
        kids.iter().fold(c, |c, k| c.child(build(s, h, *k)))
    };
    if s.memo { memo(format!("n{n}"), ver, make) } else { make() }
}

fn view(s: &G, _: &mut Cx<G>) -> El {
    let app = s.app.borrow();
    let h = app.host.borrow();
    build(s, &h, h.root())
}

// Plans: the same as rust-solid-bench.rs, plus windowed versions of the lists.
fn lit(v: J) -> J { json!({"op":"literal","value":v}) }
fn read(s: &str) -> J { json!({"op":"read","source":s}) }
fn local(s: &str) -> J { json!({"op":"local","name":s}) }
fn get(o: J, k: &str) -> J { json!({"op":"get","object":o,"key":k}) }
fn add(a: J, b: J) -> J { json!({"op":"add","left":a,"right":b}) }
fn tx(v: J) -> J { json!({"kind":"text","value":v}) }
fn elj(tag: &str, events: J, children: Vec<J>) -> J { json!({"kind":"element","tag":tag,"events":events,"children":children}) }
fn button(steps: J) -> J { elj("button", json!([{"name":"onClick","steps":steps}]), vec![tx(lit(json!("+")))]) }
fn write(s: &str, v: J) -> J { json!({"op":"write","source":s,"value":v}) }
fn rows_seed() -> J {
    let items: Vec<J> = (0..ROWS).map(|i| json!({"op":"object","fields":[
        {"key":"id","value":lit(json!(i))},
        {"key":"label","value":lit(json!(format!("row {i}")))},
        {"key":"even","value":lit(json!(i % 2 == 0))}]})).collect();
    json!({"op":"object","fields":[{"key":"list","value":{"op":"array","items":items}}]})
}
fn plan(sources: J, memos: J, markup: Vec<J>) -> Plan {
    serde_json::from_value(json!({"version":2,"component":"B","hosts":[],"sources":sources,"memos":memos,"actions":[],"markup":markup})).unwrap()
}
fn counter() -> Plan {
    plan(json!([{"name":"n","kind":"signal","seed":lit(json!(0))}]), json!([]),
        vec![elj("div", json!([]), vec![button(json!([write("n", add(read("n"), lit(json!(1))))])), elj("span", json!([]), vec![tx(read("n"))])])])
}
fn fanout() -> Plan {
    let memos: Vec<J> = (0..FAN).map(|i| json!({"name":format!("d{i}"),"value":add(read("x"), lit(json!(i)))})).collect();
    let mut kids = vec![button(json!([write("x", add(read("x"), lit(json!(1))))]))];
    for i in 0..FAN { kids.push(elj("span", json!([]), vec![tx(read(&format!("d{i}")))])); }
    plan(json!([{"name":"x","kind":"signal","seed":lit(json!(0))}]), J::Array(memos), vec![elj("div", json!([]), kids)])
}
fn select_row() -> J {
    elj("p", json!([]), vec![tx(add(
        json!({"op":"cond","test":{"op":"eq","left":get(local("r"),"id"),"right":read("sel")},"then":lit(json!("> ")),"else":lit(json!(""))}),
        get(local("r"), "label")))])
}
fn select() -> Plan {
    plan(json!([{"name":"rows","kind":"store","seed":rows_seed()},{"name":"sel","kind":"signal","seed":lit(json!(-1))}]), json!([]),
        vec![elj("div", json!([]), vec![button(json!([write("sel", add(read("sel"), lit(json!(1))))])),
            json!({"kind":"for","each":get(read("rows"),"list"),"item":"r","key":"identity","children":[select_row()]})])])
}
fn visible() -> J {
    json!({"op":"filter","array":get(read("rows"),"list"),"param":"r","body":
        {"op":"or","left":{"op":"eq","left":read("mode"),"right":lit(json!("all"))},"right":get(local("r"),"even")}})
}
fn flip() -> J {
    write("mode", json!({"op":"cond","test":{"op":"eq","left":read("mode"),"right":lit(json!("all"))},"then":lit(json!("even")),"else":lit(json!("all"))}))
}
fn filter() -> Plan {
    plan(json!([{"name":"rows","kind":"store","seed":rows_seed()},{"name":"mode","kind":"signal","seed":lit(json!("all"))}]),
        json!([{"name":"visible","value":visible()}]),
        vec![elj("div", json!([]), vec![button(json!([flip()])),
            json!({"kind":"for","each":read("visible"),"item":"r","key":"identity","children":[elj("p", json!([]), vec![tx(get(local("r"),"label"))])]})])])
}
// Windowed: the same list, sliced to [winStart, winEnd) inside a "vlist".
fn window(list: J, row: J) -> J {
    elj("vlist", json!([]), vec![json!({"kind":"for","each":{"op":"slice","array":list,"start":read("winStart"),"end":read("winEnd")},
        "item":"r","key":"identity","children":[row]})])
}
fn win_sources(mut s: Vec<J>) -> J {
    s.push(json!({"name":"winStart","kind":"signal","seed":lit(json!(0))}));
    s.push(json!({"name":"winEnd","kind":"signal","seed":lit(json!(0))}));
    J::Array(s)
}
fn select_lazy() -> Plan {
    plan(win_sources(vec![json!({"name":"rows","kind":"store","seed":rows_seed()}),json!({"name":"sel","kind":"signal","seed":lit(json!(-1))})]),
        json!([{"name":"count","value":{"op":"length","array":get(read("rows"),"list")}}]),
        vec![elj("div", json!([]), vec![button(json!([write("sel", add(read("sel"), lit(json!(1))))])),
            window(get(read("rows"),"list"), select_row())])])
}
fn filter_lazy() -> Plan {
    plan(win_sources(vec![json!({"name":"rows","kind":"store","seed":rows_seed()}),json!({"name":"mode","kind":"signal","seed":lit(json!("all"))})]),
        json!([{"name":"visible","value":visible()},{"name":"count","value":{"op":"length","array":read("visible")}}]),
        vec![elj("div", json!([]), vec![button(json!([flip()])),
            window(read("visible"), elj("p", json!([]), vec![tx(get(local("r"),"label"))]))])])
}

fn boot(p: Plan, seen: Rc<Cell<i64>>) -> (Runtime<G>, Rc<RefCell<App>>) {
    let has_count = p.memos.iter().any(|m| m.name == "count");
    let mut app = App::mount_unlogged(p).unwrap();
    app.turn();
    let b = app.clickables()[0].0;
    let app = Rc::new(RefCell::new(app));
    let g = G { app: app.clone(), b, memo: flag("GLUE_MEMO"), virt: flag("GLUE_VIRTUAL") || flag("GLUE_LAZY"), seen,
        has_count, count: Cell::new(0), win: Cell::new((0, 0)), kids: RefCell::new((usize::MAX, 0, vec![])),
        retained: flag("GLUE_RETAINED"), ret: RefCell::new(None) };
    g.refresh();
    let retained = g.retained;
    let mut rt = Runtime::new(g, view, size(), Env::new(true, 1.0));
    if retained {
        *rt.state.ret.borrow_mut() = Some(rt.retained());
        app.borrow().host.borrow_mut().record_writes(true);
    }
    for _ in 0..240 { if !rt.frame(1.0 / 120.0) { break; } }
    (rt, app)
}

// What a windowed list must show after `n` clicks: rust-solid's whole frame
// text, given the window the glue asked for.
type Expect = fn(usize, (usize, usize)) -> String;
fn lazy_select(n: usize, (a, b): (usize, usize)) -> String {
    let sel = n as i64 - 1;
    (a..b.min(ROWS)).fold("+".to_string(), |s, i| s + if i as i64 == sel { "> " } else { "" } + &format!("row {i}"))
}
fn lazy_filter(n: usize, (a, b): (usize, usize)) -> String {
    let rows: Vec<usize> = (0..ROWS).filter(|i| n % 2 == 0 || i % 2 == 0).collect();
    rows[a.min(rows.len())..b.min(rows.len())].iter().fold("+".to_string(), |s, i| s + &format!("row {i}"))
}

fn run(name: &str, make: fn() -> Plan, n: usize, check: impl Fn(&str, usize) -> bool, lazy: Option<Expect>) {
    let n = std::env::var("GLUE_N").ok().and_then(|v| v.parse().ok()).unwrap_or(n);
    let reps = std::env::var("GLUE_REPS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let mut best = f64::MAX;
    let mut made = 0;
    let mut landed = 0;
    for _ in 0..reps {
        let seen = Rc::new(Cell::new(0));
        let (mut rt, app) = boot(make(), seen.clone());
        let b = app.borrow().clickables()[0].0;
        // A virtual row takes its list's key, k{id}, in place of its own.
        let find_b = |rt: &Runtime<G>| rt.find("b").or_else(|| rt.find(&format!("k{b}")));
        let (x, y) = find_b(&rt).expect("button");
        let t = Instant::now();
        for _ in 0..n { rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(1.0 / 120.0); }
        best = best.min(t.elapsed().as_secs_f64() * 1e6 / n as f64);
        assert_eq!(seen.get(), n as i64, "{name}: every click must land");
        assert!(find_b(&rt).is_some(), "{name}: button still mounted");
        if let Some(r) = rt.state.ret.borrow().as_ref() { landed = r.landed(); }
        let frame = app.borrow().frame();
        match lazy {
            Some(expect) => {
                let a = int(app.borrow_mut().value("winStart").unwrap()) as usize;
                let b = int(app.borrow_mut().value("winEnd").unwrap()) as usize;
                let win = (a, b);
                assert!(win.1 > win.0 && win.0 == 0, "{name}: window {win:?}");
                assert_eq!(frame, expect(n, win), "{name}: wrong frame after {n} clicks");
                // Retained rows take their text's key instead (see row_key).
                assert!(rt.state.retained || rt.find("i0").is_some(), "{name}: first row on screen");
                made = made.max(frame.matches("row ").count());
            }
            None => assert!(check(&frame, n), "{name}: wrong frame after {n} clicks"),
        }
        drop(rt);
        app.borrow_mut().dispose();
    }
    let tag = match (flag("GLUE_LAZY"), flag("GLUE_MEMO"), flag("GLUE_VIRTUAL")) {
        (true, false, _) => "lazy", (true, true, _) => "lazy + memo",
        (false, false, false) => "rebuild", (false, true, false) => "memo per node",
        (false, false, true) => "virtual lists", (false, true, true) => "virtual lists + memo",
    };
    let tag = if flag("GLUE_RETAINED") { format!("retained + {tag}") } else { tag.to_string() };
    let mut rows = if made > 0 { format!(", {made} rows made") } else { String::new() };
    if flag("GLUE_RETAINED") { rows += &format!(", {landed} of {n} clicks skipped the view"); }
    println!("glue {name:<10} {best:>10.2} us/click  {tag} (n={n}, best of {reps}{rows})");
}

// Plain Rune, as solid_bench.rs writes the workloads: the reference for GLUE_CHECK.
mod plain {
    use super::*;
    #[derive(Default)]
    pub struct S { n: i64, even_only: bool }
    const R: i64 = ROWS as i64;
    fn button() -> El { el().key("b").size(40.0, 20.0).on_press(|s: &mut S| { s.n += 1; s.even_only = !s.even_only }) }
    fn row(i: i64, sel: bool) -> El { text(format!("{}row {i}", if sel { "> " } else { "" })).key(format!("r{i}")) }
    pub fn counter(s: &S, _: &mut Cx<S>) -> El { col().child(button()).child(text(format!("{}", s.n))) }
    pub fn fanout(s: &S, _: &mut Cx<S>) -> El { (0..FAN as i64).fold(col().child(button()), |c, i| c.child(text(format!("{}", s.n + i)))) }
    pub fn select(s: &S, _: &mut Cx<S>) -> El { (0..R).fold(col().child(button()), |c, i| c.child(row(i, i == s.n - 1))) }
    pub fn filter(s: &S, _: &mut Cx<S>) -> El {
        (0..R).filter(|i| !s.even_only || i % 2 == 0).fold(col().child(button()), |c, i| c.child(row(i, false)))
    }
    fn vlist(count: usize, version: u64, key: fn(&S, usize) -> Cow<'_, str>, build: fn(&S, usize) -> El) -> El {
        col().grow(1.0).child(button()).child(col().scroll(ScrollSpec { stick_bottom: false }).grow(1.0).child(
            col().key("rows").growth(GrowthSpec::default()).virtual_rows(count, 20.0, key, build).virtual_rows_version(version)))
    }
    pub fn fanout_v(_: &S, _: &mut Cx<S>) -> El {
        vlist(FAN, 0, |_, i| Cow::Owned(format!("f{i}")), |s, i| text(format!("{}", s.n + i as i64)))
    }
    pub fn select_v(_: &S, _: &mut Cx<S>) -> El {
        vlist(ROWS, 0, |_, i| Cow::Owned(format!("r{i}")), |s, i| row(i as i64, i as i64 == s.n - 1))
    }
    pub fn filter_v(s: &S, _: &mut Cx<S>) -> El {
        let count = if s.even_only { ROWS / 2 } else { ROWS };
        vlist(count, s.even_only as u64, |s, i| Cow::Owned(format!("r{}", if s.even_only { 2 * i } else { i })),
            |s, i| row(if s.even_only { 2 * i as i64 } else { i as i64 }, false))
    }
}

/// The text Rune shows in the window, top to bottom.
fn shown<St: 'static>(rt: &Runtime<St>) -> Vec<String> {
    let mut v: Vec<_> = rt.elements().into_iter()
        .filter(|e| !e.exiting && e.visible[2] > 0.0 && e.visible[3] > 0.0)
        .filter_map(|e| Some((e.rect[1], e.rect[0], e.text?)))
        .collect();
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    v.into_iter().map(|t| t.2).collect()
}

fn check(name: &str, make: fn() -> Plan, view: fn(&plain::S, &mut Cx<plain::S>) -> El) {
    let (mut rt, app) = boot(make(), Rc::new(Cell::new(0)));
    let b = app.borrow().clickables()[0].0;
    let find_b = |rt: &Runtime<G>| rt.find("b").or_else(|| rt.find(&format!("k{b}")));
    let mut p = Runtime::new(plain::S::default(), view, size(), Env::new(true, 1.0));
    for _ in 0..240 { if !p.frame(1.0 / 120.0) { break; } }
    let (px, py) = p.find("b").expect("plain button");
    assert_eq!(shown(&rt), shown(&p), "{name}: before any click");
    for k in 1..=60 {
        let (x, y) = find_b(&rt).expect("button");
        // Both settle first: keyed rows that move slide in plain Rune, while the
        // windowed glue keys rows by slot.
        rt.pointer_down(x, y); rt.pointer_up(x, y);
        for _ in 0..240 { if !rt.frame(1.0 / 120.0) { break; } }
        p.pointer_down(px, py); p.pointer_up(px, py);
        for _ in 0..240 { if !p.frame(1.0 / 120.0) { break; } }
        let (a, b) = (shown(&rt), shown(&p));
        assert!(!a.is_empty(), "{name}: nothing shown after click {k}");
        assert_eq!(a, b, "{name}: differs from plain Rune after click {k}");
    }
    let n = shown(&rt).len();
    drop(rt);
    app.borrow_mut().dispose();
    println!("check {name:<10} same text as plain Rune after each of 60 clicks ({n} texts in view)");
}

fn main() {
    if cfg!(debug_assertions) { eprintln!("debug build: time a --profile perf build"); }
    if flag("GLUE_CHECK") {
        let v = flag("GLUE_VIRTUAL") || flag("GLUE_LAZY");
        let lazy = flag("GLUE_LAZY");
        check("counter", counter, plain::counter);
        check("fanout", fanout, if v { plain::fanout_v } else { plain::fanout });
        check("select", if lazy { select_lazy } else { select }, if v { plain::select_v } else { plain::select });
        check("filter", if lazy { filter_lazy } else { filter }, if v { plain::filter_v } else { plain::filter });
        return;
    }
    let lazy = flag("GLUE_LAZY");
    let only = std::env::args().nth(1);
    let want = |w: &str| only.as_deref().is_none_or(|o| o == w);
    if want("counter") { run("counter", counter, 100_000, |f, n| f.ends_with(&n.to_string()), None); }
    if want("fanout") { run("fanout", fanout, 2_000, |f, n| f.contains(&(n + FAN - 1).to_string()), None); }
    if want("select") {
        if lazy { run("select", select_lazy, 2_000, |_, _| true, Some(lazy_select)); }
        else { run("select", select, 2_000, |f, n| f.contains(&format!("> row {}", (n - 1) % ROWS)) || n > ROWS, None); }
    }
    if want("filter") {
        if lazy { run("filter", filter_lazy, 1_000, |_, _| true, Some(lazy_filter)); }
        else { run("filter", filter, 1_000, |f, _| f.contains("row 1"), None); }
    }
    if want("mount") {
        let mut best = f64::MAX;
        let p = if lazy { select_lazy() } else { select() };
        for _ in 0..5 {
            let t = Instant::now();
            for _ in 0..100 { let (rt, app) = boot(p.clone(), Rc::new(Cell::new(0))); drop(rt); app.borrow_mut().dispose(); }
            best = best.min(t.elapsed().as_secs_f64() * 1e6 / 100.0);
        }
        println!("glue {:<10} {best:>10.2} us/mount  rust-solid mount+turn, Rune boot+layout, drop ({ROWS} rows)", "mount");
    }
}
