// Scratch benchmark, run on Nov 10 from devagrawal09/rune 29aacb68 (results in
// PERF.md; linking needs libxkbcommon-dev). Same five workloads as rust-solid-bench.rs and
// exact-bench.rs, through Rune's headless Runtime (no window, no GPU), the way
// crates/rune/examples/interaction_cost.rs drives it. One Runtime::frame after
// a click runs the queued handler, re-runs the view, diffs by key, lays out
// with Taffy and shapes changed text, so the figure compares with Exact's
// "commit + layout" column, not with rust-solid's (which has no layout).
// The window is 400x800; TALL=1 makes it 400x100000 (the first round's size,
// where every row is on screen). The "/v" variants use Rune's virtual_rows.
//
// Run as an example of a clone of devagrawal09/rune:
//   cp rune-bench.rs crates/rune/examples/solid_bench.rs
//   cargo run --profile perf -p rune --no-default-features --features embedded-fonts --example solid_bench
//
// Each workload has a plain view and, where it matters, a memo'd one (Rune's
// own answer to "skip what didn't change"), so both idioms are on record.
use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use std::borrow::Cow;

use rune::{Cx, El, Env, GrowthSpec, Runtime, ScrollSpec, col, el, memo, text};

const ROWS: i64 = 1000;
const FAN: i64 = 1000;
fn size() -> (f32, f32) { if std::env::var("TALL").is_ok() { (400.0, 100_000.0) } else { (400.0, 800.0) } }

#[derive(Default)]
struct S { n: i64, even_only: bool, memo_rows: bool, seen: Rc<Cell<i64>> }

fn button<St: 'static>(f: fn(&mut St)) -> El { el().key("b").size(40.0, 20.0).on_press(f) }

fn counter(s: &S, _: &mut Cx<S>) -> El {
    col().child(button(|s: &mut S| { s.n += 1; s.seen.set(s.n) })).child(text(format!("{}", s.n)))
}

fn fanout(s: &S, _: &mut Cx<S>) -> El {
    (0..FAN).fold(col().child(button(|s: &mut S| { s.n += 1; s.seen.set(s.n) })), |c, i| c.child(text(format!("{}", s.n + i))))
}

fn row(i: i64, sel: bool) -> El { text(format!("{}row {i}", if sel { "> " } else { "" })).key(format!("r{i}")) }

fn select(s: &S, _: &mut Cx<S>) -> El {
    (0..ROWS).fold(col().child(button(|s: &mut S| { s.n += 1; s.seen.set(s.n) })), |c, i| {
        let sel = i == s.n - 1;
        c.child(if s.memo_rows { memo(format!("m{i}"), sel, move || row(i, sel)) } else { row(i, sel) })
    })
}

fn filter(s: &S, _: &mut Cx<S>) -> El {
    (0..ROWS).filter(|i| !s.even_only || i % 2 == 0).fold(
        col().child(button(|s: &mut S| { s.even_only = !s.even_only; s.n += 1; s.seen.set(s.n) })),
        |c, i| c.child(if s.memo_rows { memo(format!("m{i}"), 0u8, move || row(i, false)) } else { row(i, false) }),
    )
}

// Rune's own idiom for a long list: virtual_rows builds and lays out only the
// rows in the mounted window (the viewport plus up to two either side).
fn vlist(b: El, count: usize, version: u64, key: fn(&S, usize) -> Cow<'_, str>, build: fn(&S, usize) -> El) -> El {
    col().grow(1.0).child(b).child(col().scroll(ScrollSpec { stick_bottom: false }).grow(1.0).child(
        col().key("rows").growth(GrowthSpec::default()).virtual_rows(count, 20.0, key, build).virtual_rows_version(version)))
}

fn fanout_v(_: &S, _: &mut Cx<S>) -> El {
    vlist(button(|s: &mut S| { s.n += 1; s.seen.set(s.n) }), FAN as usize, 0,
        |_, i| Cow::Owned(format!("f{i}")), |s, i| text(format!("{}", s.n + i as i64)))
}

fn select_v(_: &S, _: &mut Cx<S>) -> El {
    vlist(button(|s: &mut S| { s.n += 1; s.seen.set(s.n) }), ROWS as usize, 0,
        |_, i| Cow::Owned(format!("r{i}")), |s, i| {
            let (i, sel) = (i as i64, i as i64 == s.n - 1);
            if s.memo_rows { memo(format!("m{i}"), sel, move || row(i, sel)) } else { row(i, sel) }
        })
}

fn filter_v(s: &S, _: &mut Cx<S>) -> El {
    let count = if s.even_only { ROWS as usize / 2 } else { ROWS as usize };
    vlist(button(|s: &mut S| { s.even_only = !s.even_only; s.n += 1; s.seen.set(s.n) }), count, s.even_only as u64,
        |s, i| Cow::Owned(format!("r{}", if s.even_only { 2 * i } else { i })), |s, i| {
            let i = if s.even_only { 2 * i as i64 } else { i as i64 };
            if s.memo_rows { memo(format!("m{i}"), 0u8, move || row(i, false)) } else { row(i, false) }
        })
}

fn boot(view: fn(&S, &mut Cx<S>) -> El, memo_rows: bool, seen: Rc<Cell<i64>>) -> Runtime<S> {
    let mut rt = Runtime::new(S { memo_rows, seen, ..S::default() }, view, size(), Env::new(true, 1.0));
    for _ in 0..240 { if !rt.frame(1.0 / 120.0) { break; } }
    rt
}

fn run(name: &str, view: fn(&S, &mut Cx<S>) -> El, memo_rows: bool, n: usize) {
    let mut best = f64::MAX;
    for _ in 0..5 {
        let seen = Rc::new(Cell::new(0));
        let mut rt = boot(view, memo_rows, seen.clone());
        let (x, y) = rt.find("b").expect("button");
        let t = Instant::now();
        for _ in 0..n { rt.pointer_down(x, y); rt.pointer_up(x, y); rt.frame(1.0 / 120.0); }
        best = best.min(t.elapsed().as_secs_f64() * 1e6 / n as f64);
        assert_eq!(seen.get(), n as i64, "{name}: every click must land");
    }
    let tag = if memo_rows { "memo rows" } else { "plain" };
    println!("rune {name:<10} {best:>10.2} us/click  commit+layout, {tag} (n={n}, best of 5)");
}

fn main() {
    if cfg!(debug_assertions) { eprintln!("debug build: time a --profile perf build"); }
    run("counter", counter, false, 100_000);
    run("fanout", fanout, false, 2_000);
    for m in [false, true] { run("select", select, m, 2_000); run("filter", filter, m, 1_000); }
    run("fanout/v", fanout_v, false, 2_000);
    for m in [false, true] { run("select/v", select_v, m, 2_000); run("filter/v", filter_v, m, 1_000); }
    for (name, v) in [("mount", select as fn(&S, &mut Cx<S>) -> El), ("mount/v", select_v)] {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            for _ in 0..100 { drop(boot(v, false, Rc::new(Cell::new(0)))); }
            best = best.min(t.elapsed().as_secs_f64() * 1e6 / 100.0);
        }
        println!("rune {name:<10} {best:>10.2} us/mount  boot+layout+drop ({ROWS} rows, best of 5)");
    }
}
