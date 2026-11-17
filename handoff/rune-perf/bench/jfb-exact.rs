// jfb keyed on Exact: the table as a contract (state, actions, a keyed for),
// compiled once, driven by Runner::dispatch(Press) on the header buttons and
// on each row's links. Timed region: the dispatch (plan VM, kernel apply),
// and with EXACT_LAYOUT also a Taffy layout of the root at 400 x 800.
// Exact's lists don't change in place, so each action builds the new list
// (`rows.map`, `concat`, `filter`); the label is a contract `fn` of the id.
// Run as an example of exact-runner (dev-dependency `contract`):
//   cargo run --release -p exact-runner --example jfb -- [op|all] [reps]
#[path = "jfb_common.rs"]
mod common;
use common::{Driver, Shown};
use exact_kernel::{Kernel, NodeRef, Offer, ViewId};
use exact_plan::{Plan, Value};
use exact_runner::{DataError, DataSource, Event, Runner};

struct NoData;
impl DataSource for NoData {
    fn query(&mut self, name: &str, _: &[Value]) -> Result<Value, DataError> {
        Err(DataError::UnknownSource(name.into()))
    }
}

fn words(xs: &[&str]) -> String { format!("[{}]", xs.iter().map(|w| format!("\"{w}\"")).collect::<Vec<_>>().join(", ")) }

fn source() -> String {
    let adj = words(&["pretty", "large", "big", "small", "tall", "short", "long", "handsome", "plain", "quaint", "clean",
        "elegant", "easy", "angry", "crazy", "helpful", "mushy", "odd", "unsightly", "adorable", "important", "inexpensive",
        "cheap", "expensive", "fancy"]);
    let colour = words(&["red", "yellow", "blue", "green", "pink", "brown", "purple", "brown", "white", "black", "orange"]);
    let noun = words(&["table", "chair", "house", "bbq", "desk", "car", "pony", "cookie", "sandwich", "burger", "pizza",
        "mouse", "keyboard"]);
    format!(r#"shape Row
  id: number
  label: string

fn word(xs: string[], i: number): string = match xs.at(i) {{ case some(w) => w, case none => "" }}
fn hash(id: number): number = (id * 2654435761) % 4294967296
fn words(h: number): string = `${{word({adj}, h % 25)}} ${{word({colour}, floor(h / 25) % 11)}} ${{word({noun}, floor(h / 275) % 13)}}`
fn make(id: number): Row = Row(id=id, label=words(hash(id)))
fn swapped(rows: Row[], i: number, j: number): Row[] = match rows.at(i) {{
  case some(a) => match rows.at(j) {{ case some(b) => rows.map((r, k) => k == i ? b : (k == j ? a : r)), case none => rows }},
  case none => rows
}}

component App
  state rows: Row[] = []
  state next = 1
  state sel = -1
  action run
    rows = range(1000).map(i => make(next + i))
    next = next + 1000
  action runlots
    rows = range(10000).map(i => make(next + i))
    next = next + 10000
  action add
    rows = rows.concat(range(1000).map(i => make(next + i)))
    next = next + 1000
  action update
    rows = rows.map((r, i) => i % 10 == 0 ? Row(r, label=`${{r.label}} !!!`) : r)
  action clear
    rows = []
  action swaprows
    rows = rows.length > 998 ? swapped(rows, 1, 998) : rows
  action select(id: number)
    sel = id
  action remove(id: number)
    rows = rows.filter(r => r.id != id)
  view
    column
      button press=run testId="run"
        text "Create 1,000 rows"
      button press=runlots testId="runlots"
        text "Create 10,000 rows"
      button press=add testId="add"
        text "Append 1,000 rows"
      button press=update testId="update"
        text "Update every 10th row"
      button press=clear testId="clear"
        text "Clear"
      button press=swaprows testId="swaprows"
        text "Swap Rows"
      column testId="tbody"
        for r in rows key=r.id
          row
            text (r.id == sel ? "danger" : "-")
            text `${{r.id}}`
            button press=select(r.id) testId=`select-${{r.id}}`
              text r.label
            button press=remove(r.id) testId=`remove-${{r.id}}` aria-label="Remove" width=8 height=8
            column
"#)
}

struct Ex { r: Runner<NoData>, layout: bool }

impl Ex {
    fn view(&self, test_id: &str) -> ViewId {
        let k = self.r.kernel().find_first_by_test_id(test_id).unwrap_or_else(|| panic!("no {test_id}"));
        self.r.kernel().node_by_key(k).unwrap().id
    }
    fn press(&mut self, test_id: &str) {
        let v = self.view(test_id);
        self.r.dispatch(v, Event::Press).unwrap();
        if self.layout {
            let root = self.r.kernel().roots()[0];
            self.r.kernel_mut().compute_layout(root, Offer::definite(400.0, 800.0)).unwrap();
        }
    }
    /// Row `i`'s id, as its second cell shows it.
    fn row_id(&self, i: usize) -> String {
        let body = self.view("tbody");
        let tr = self.r.kernel().node(body).unwrap().child_nodes().nth(i).expect("row");
        text(&tr.child_nodes().nth(1).unwrap())
    }
}

fn text(n: &NodeRef<'_>) -> String {
    let runs: String = n.text_runs().iter().map(|r| &*r.text).collect();
    if !runs.is_empty() { return runs; }
    n.child_nodes().map(|c| text(&c)).collect()
}

impl Driver for Ex {
    fn run(&mut self) { self.press("run") }
    fn runlots(&mut self) { self.press("runlots") }
    fn add(&mut self) { self.press("add") }
    fn update(&mut self) { self.press("update") }
    fn clear(&mut self) { self.press("clear") }
    fn swap(&mut self) { self.press("swaprows") }
    fn select(&mut self, i: usize) { let id = self.row_id(i); self.press(&format!("select-{id}")) }
    fn remove(&mut self, i: usize) { let id = self.row_id(i); self.press(&format!("remove-{id}")) }
    fn shown(&mut self) -> Shown {
        let body = self.view("tbody");
        let k: &Kernel = self.r.kernel();
        let rows = k.node(body).unwrap().child_nodes().map(|tr| {
            let c: Vec<String> = tr.child_nodes().map(|c| text(&c)).collect();
            (c[0].clone(), c[1].clone(), c[2].clone())
        }).collect();
        Shown { rows, windowed: false }
    }
}

fn main() {
    let layout = std::env::var("EXACT_LAYOUT").is_ok();
    let plan: Plan = contract::compile(&source()).unwrap_or_else(|e| panic!("contract: {e:?}"));
    common::main_with(if layout { "exact-layout" } else { "exact" }, || {
        let mut r = Runner::boot(plan.clone(), NoData, Kernel::with_monospace(), Default::default(), "/").unwrap();
        r.measure(false, false);
        if layout {
            let root = r.kernel().roots()[0];
            r.kernel_mut().compute_layout(root, Offer::definite(400.0, 800.0)).unwrap();
        }
        Ex { r, layout }
    });
}
