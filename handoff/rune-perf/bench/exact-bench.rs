// Scratch benchmark (not part of the repo): the same five workloads as the
// rust-solid bench, driven through Runner::boot / dispatch(Press). Each
// workload is timed twice: the commit alone (plan VM + kernel apply), and the
// commit plus a Taffy layout of the root, which rust-solid has no counterpart for.
use exact_kernel::{Kernel, Offer, ViewId};
use exact_plan::{Plan, Value};
use exact_runner::{DataError, DataSource, Event, Runner};
use std::fmt::Write;
use std::time::Instant;

const ROWS: usize = 1000;
const FAN: usize = 1000;

struct NoData;
impl DataSource for NoData {
    fn query(&mut self, name: &str, _: &[Value]) -> Result<Value, DataError> {
        Err(DataError::UnknownSource(name.into()))
    }
}

const ROW_SHAPE: &str = "shape Row\n  id: number\n  label: string\n  even: bool\n";
const ROWS_STATE: &str = "  state rows: Row[] = range(1000).map(i => Row(id=i, label=`row ${i}`, even=floor(i / 2) * 2 == i))\n";

fn counter() -> String {
    "component App\n  state n = 0\n  action inc\n    n = n + 1\n  view\n    column\n      button press=inc testId=\"b\"\n        text \"+\"\n      text `${n}`\n".into()
}

fn fanout() -> String {
    let mut s = String::from("component App\n  state x = 0\n  action inc\n    x = x + 1\n");
    for i in 0..FAN { writeln!(s, "  derive d{i} = x + {i}").unwrap(); }
    s.push_str("  view\n    column\n      button press=inc testId=\"b\"\n        text \"+\"\n");
    for i in 0..FAN { writeln!(s, "      text `${{d{i}}}`").unwrap(); }
    s
}

fn select() -> String {
    format!("{ROW_SHAPE}component App\n{ROWS_STATE}  state sel = -1\n  action next\n    sel = sel + 1\n  view\n    column\n      button press=next testId=\"b\"\n        text \"+\"\n      for r in rows key=r.id\n        text `${{r.id == sel ? \"> \" : \"\"}}${{r.label}}`\n")
}

fn filter() -> String {
    format!("{ROW_SHAPE}component App\n{ROWS_STATE}  state mode = \"all\"\n  derive visible = rows.filter(r => mode == \"all\" || r.even)\n  derive count = visible.length\n  action flip\n    mode = mode == \"all\" ? \"even\" : \"all\"\n  view\n    column\n      button press=flip testId=\"b\"\n        text \"+\"\n      for r in visible key=r.id\n        text r.label\n")
}

fn boot(plan: &Plan) -> Runner<NoData> {
    let mut r = Runner::boot(plan.clone(), NoData, Kernel::with_monospace(), Default::default(), "/").unwrap();
    r.measure(false, false);
    r
}

fn button(r: &Runner<NoData>) -> ViewId {
    let key = r.kernel().find_first_by_test_id("b").unwrap();
    r.kernel().node_by_key(key).unwrap().id
}

fn layout(r: &mut Runner<NoData>) {
    let root = r.kernel().roots()[0];
    r.kernel_mut().compute_layout(root, Offer::definite(400.0, 100_000.0)).unwrap();
}

fn run(name: &str, src: String, n: usize, check: impl Fn(&Runner<NoData>, usize)) {
    let plan = contract::compile(&src).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    for with_layout in [false, true] {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let mut r = boot(&plan);
            if with_layout { layout(&mut r); }
            let b = button(&r);
            let t = Instant::now();
            for _ in 0..n {
                r.dispatch(b, Event::Press).unwrap();
                if with_layout { layout(&mut r); }
            }
            best = best.min(t.elapsed().as_secs_f64() * 1e6 / n as f64);
            check(&r, n);
        }
        let what = if with_layout { "commit+layout" } else { "commit" };
        println!("exact {name:<10} {best:>10.2} us/click  {what} (n={n}, best of 5)");
    }
}

fn int(r: &Runner<NoData>, d: &str) -> f64 {
    match r.derive(d).or_else(|| r.slot(d)) { Some(Value::Number(x)) => *x, other => panic!("{d}: {other:?}") }
}

fn main() {
    run("counter", counter(), 100_000, |r, n| assert_eq!(int(r, "n") as usize, n));
    run("fanout", fanout(), 2_000, |r, n| assert_eq!(int(r, "d999") as usize, n + FAN - 1));
    run("select", select(), 2_000, |r, n| assert_eq!(int(r, "sel") as usize, n - 1));
    run("filter", filter(), 1_000, |r, n| assert_eq!(int(r, "count") as usize, if n % 2 == 0 { ROWS } else { ROWS / 2 }));
    let plan = contract::compile(&select()).unwrap();
    for with_layout in [false, true] {
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            for _ in 0..100 { let mut r = boot(&plan); if with_layout { layout(&mut r); } drop(r); }
            best = best.min(t.elapsed().as_secs_f64() * 1e6 / 100.0);
        }
        let what = if with_layout { "boot+layout+drop" } else { "boot+drop" };
        println!("exact {:<10} {best:>10.2} us/mount  {what} ({ROWS} rows, plan prebuilt, best of 5)", "mount");
    }
}
