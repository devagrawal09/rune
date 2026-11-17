// js-framework-benchmark (krausest), keyed, ported to headless in-process
// workloads. Shared by every runtime's bench: the row data, a model of what the
// table must show, the nine operations with jfb's own warmups, and the check
// after each run. Each runtime supplies a Driver whose methods include their
// own settle (flush, frame), so the timed region is one operation end to end.
//
// Deviations from jfb, the same for every runtime:
// - Labels come from a hash of the row id, not Math.random, so every runtime
//   builds the same table: the same word lists, picked by
//   h = id * 2654435761 mod 2^32, adjective h % 25, colour (h / 25) % 11,
//   noun (h / 275) % 13.
// - The selected row shows a text cell "danger" ("-" otherwise) instead of a
//   class, because rust-solid's plan v2 has no attributes.
// - Operations run through each runtime's own handler path, without a pointer
//   hit test, and nothing paints.
#![allow(dead_code)]
use std::time::Instant;

const ADJ: [&str; 25] = ["pretty", "large", "big", "small", "tall", "short", "long", "handsome", "plain", "quaint", "clean",
    "elegant", "easy", "angry", "crazy", "helpful", "mushy", "odd", "unsightly", "adorable", "important", "inexpensive",
    "cheap", "expensive", "fancy"];
const COLOUR: [&str; 11] = ["red", "yellow", "blue", "green", "pink", "brown", "purple", "brown", "white", "black", "orange"];
const NOUN: [&str; 13] = ["table", "chair", "house", "bbq", "desk", "car", "pony", "cookie", "sandwich", "burger", "pizza",
    "mouse", "keyboard"];

pub fn label(id: u64) -> String {
    let h = id.wrapping_mul(2654435761) & 0xffff_ffff;
    format!("{} {} {}", ADJ[(h % 25) as usize], COLOUR[((h / 25) % 11) as usize], NOUN[((h / 275) % 13) as usize])
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: u64,
    pub label: String,
}

/// Fresh rows from `next`, as jfb's buildData.
pub fn build(next: &mut u64, n: usize) -> Vec<Row> {
    (0..n).map(|_| { let id = *next; *next += 1; Row { id, label: label(id) } }).collect()
}

/// What the table must show.
#[derive(Default)]
pub struct Model { rows: Vec<Row>, next: u64, sel: Option<u64> }

impl Model {
    pub fn new() -> Self { Model { next: 1, ..Default::default() } }
    fn apply(&mut self, s: Step) {
        match s {
            Step::Run => self.rows = build(&mut self.next, 1000),
            Step::RunLots => self.rows = build(&mut self.next, 10_000),
            Step::Add => { let more = build(&mut self.next, 1000); self.rows.extend(more) }
            Step::Update => for i in (0..self.rows.len()).step_by(10) { self.rows[i].label += " !!!" },
            Step::Clear => self.rows.clear(),
            Step::Swap => if self.rows.len() > 998 { self.rows.swap(1, 998) },
            Step::Select(i) => self.sel = Some(self.rows[i].id),
            Step::Remove(i) => { self.rows.remove(i); }
        }
    }
    /// The table as (selected marker, id, label) rows.
    fn table(&self) -> Vec<(String, String, String)> {
        self.rows.iter().map(|r| (if self.sel == Some(r.id) { "danger" } else { "-" }.to_string(), r.id.to_string(), r.label.clone())).collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Step { Run, RunLots, Add, Update, Clear, Swap, Select(usize), Remove(usize) }

/// What a runtime shows: its rows top to bottom as (marker, id, label), and
/// whether it shows only a window of the table (a virtual list).
pub struct Shown { pub rows: Vec<(String, String, String)>, pub windowed: bool }

pub trait Driver {
    fn run(&mut self);
    fn runlots(&mut self);
    fn add(&mut self);
    fn update(&mut self);
    fn clear(&mut self);
    fn swap(&mut self);
    fn select(&mut self, row: usize);
    fn remove(&mut self, row: usize);
    fn shown(&mut self) -> Shown;
}

fn step<D: Driver>(d: &mut D, s: Step) {
    match s {
        Step::Run => d.run(),
        Step::RunLots => d.runlots(),
        Step::Add => d.add(),
        Step::Update => d.update(),
        Step::Clear => d.clear(),
        Step::Swap => d.swap(),
        Step::Select(i) => d.select(i),
        Step::Remove(i) => d.remove(i),
    }
}

/// jfb's init (warmups included) and timed step for each keyed operation.
pub fn plan(op: &str) -> (Vec<Step>, Step) {
    use Step::*;
    let cycles = |n: usize| (0..n).flat_map(|_| [Run, Clear]).collect::<Vec<_>>();
    match op {
        "run" => (cycles(5), Run),
        "replace" => (vec![Run; 5], Run),
        "update" => (vec![Run, Update, Update, Update], Update),
        "select" => (vec![Run, Select(4)], Select(1)),
        "swap" => ([vec![Run], vec![Swap; 6]].concat(), Swap),
        "remove" => (vec![Run, Remove(8), Remove(7), Remove(6), Remove(5), Remove(4)], Remove(3)),
        "runlots" => (cycles(5), RunLots),
        "add" => ([cycles(5), vec![Run]].concat(), Add),
        "clear" => ([cycles(5), vec![Run]].concat(), Clear),
        _ => panic!("unknown op {op}"),
    }
}

pub const OPS: [&str; 9] = ["run", "replace", "update", "select", "swap", "remove", "runlots", "add", "clear"];

/// Fails unless the runtime shows the model's table: all of it, or for a
/// windowed list a slice that starts at the top and covers the viewport.
fn check(name: &str, op: &str, shown: Shown, m: &Model) {
    let want = m.table();
    if !shown.windowed {
        if shown.rows != want {
            let at = shown.rows.iter().zip(&want).position(|(a, b)| a != b);
            panic!("{name} {op}: shows {} rows, wants {}; first difference at {at:?}: {:?} vs {:?}", shown.rows.len(), want.len(),
                at.map(|i| &shown.rows[i]), at.map(|i| &want[i]));
        }
        return;
    }
    let n = shown.rows.len();
    assert!(n <= want.len() && n >= want.len().min(30), "{name} {op}: window of {n} rows for a table of {}", want.len());
    assert_eq!(shown.rows[..], want[..n], "{name} {op}: window differs from the table's top");
}

/// One sample per rep: a fresh runtime, jfb's untimed init, then the timed
/// step. Both ends are checked. Returns microseconds.
pub fn bench<D: Driver>(name: &str, op: &str, reps: usize, mut make: impl FnMut() -> D) -> Vec<f64> {
    let (init, timed) = plan(op);
    let mut out = vec![];
    for _ in 0..reps {
        let mut d = make();
        let mut m = Model::new();
        for s in &init { step(&mut d, *s); m.apply(*s); }
        check(name, op, d.shown(), &m);
        let t = Instant::now();
        step(&mut d, timed);
        out.push(t.elapsed().as_secs_f64() * 1e6);
        m.apply(timed);
        check(name, op, d.shown(), &m);
        drop(d);
    }
    out
}

/// `<bin> <op> <reps>`: one JSON line of samples per op.
pub fn main_with<D: Driver>(name: &str, make: impl Fn() -> D) {
    if cfg!(debug_assertions) { eprintln!("debug build: time a release build"); }
    let args: Vec<String> = std::env::args().collect();
    let reps: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(15);
    let ops: Vec<&str> = match args.get(1).map(|s| s.as_str()) { None | Some("all") => OPS.to_vec(), Some(o) => vec![o] };
    for op in ops {
        let s = bench(name, op, reps, &make);
        let list = s.iter().map(|x| format!("{x:.1}")).collect::<Vec<_>>().join(",");
        println!("{{\"runtime\":\"{name}\",\"op\":\"{op}\",\"us\":[{list}]}}");
    }
}
