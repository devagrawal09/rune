//! The Solid side of a window: a mounted plan, the backend that answers its host calls on the
//! window's clock, and the writes the plan's tree made since the last frame.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use rune::{Retained, TextEdit, Tx};
use rust_solid::plan_v2::{App, Plan, json_value};
use rust_solid::renderer::{HostWrite, TreeRenderer};
use serde_json::Value as J;

/// A host node id in the plan's tree.
pub type Node = usize;

/// Answers the plan's host calls (`api.d.rx`): the result, and how long it takes to arrive.
pub trait Backend {
    fn call(&mut self, host: &str, args: &[J]) -> Reply;
}

pub struct Reply {
    /// Seconds on the window's clock before the call settles.
    pub after: f64,
    pub result: Result<J, String>,
}

impl Reply {
    pub fn ok(after: f64, value: J) -> Reply {
        Reply { after, result: Ok(value) }
    }
    pub fn fail(after: f64, why: impl Into<String>) -> Reply {
        Reply { after, result: Err(why.into()) }
    }
}

/// One host call as the activity strip shows it.
pub struct Call {
    pub host: String,
    pub args: String,
    pub started: f64,
    pub settled: Option<(f64, String)>,
}

struct Flight {
    due: f64,
    host: String,
    number: usize,
    result: Result<J, String>,
    call: usize,
}

pub struct Solid {
    pub(crate) app: App,
    backend: Box<dyn Backend>,
    seen: usize,
    flights: Vec<Flight>,
    pub(crate) inputs: HashMap<Node, TextEdit>,
    pub(crate) calls: Vec<Call>,
    pub(crate) error: Option<String>,
    pub(crate) retained: RefCell<Option<Retained>>,
    /// Views run and commits that only wrote text, for the strip.
    pub(crate) views: Cell<u64>,
    pub(crate) shown: RefCell<Vec<String>>,
}

/// How many host calls the strip lists.
pub const STRIP: usize = 5;

impl Solid {
    /// Mounts a plan (`resolvePlanV2` output) against a backend.
    pub fn new(plan: &str, backend: Box<dyn Backend>) -> Solid {
        let (app, error) = match mount(plan) {
            Ok(app) => (app, None),
            Err(e) => (empty(), Some(e)),
        };
        app.host.borrow_mut().record_writes(true);
        let mut s = Solid {
            app,
            backend,
            seen: 0,
            flights: Vec::new(),
            inputs: HashMap::new(),
            calls: Vec::new(),
            error,
            retained: RefCell::new(None),
            views: Cell::new(0),
            shown: RefCell::new(vec![String::new(); STRIP + 1]),
        };
        s.sync_inputs();
        s
    }

    /// A press on `node`, as the DOM's `click`.
    pub fn click(&mut self, node: Node) {
        let r = self.app.click(node);
        self.note(r);
    }

    /// An event of type `ty` at `node`; `input` carries the field's new text.
    pub fn dispatch(&mut self, node: Node, ty: &str, value: Option<&str>) {
        let r = self.app.dispatch(node, ty, value);
        self.note(r);
    }

    fn note(&mut self, r: Result<(), String>) {
        if let Err(e) = r {
            self.error = Some(e);
        }
    }

    pub(crate) fn host(&self) -> std::cell::Ref<'_, TreeRenderer> {
        self.app.host.borrow()
    }

    /// The field's edit, created from the node's `value` the first time.
    pub(crate) fn edit(&mut self, node: Node) -> &mut TextEdit {
        let value = prop_text(&self.host(), node, "value");
        self.inputs.entry(node).or_insert_with(|| TextEdit::new(value))
    }

    /// Issues new host calls, settles the due ones, runs a turn and mirrors what the tree
    /// wrote. True when the view has to run; text-only changes go through [`Retained`].
    pub fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut strip = false;
        for _ in 0..8 {
            strip |= self.issue(clock);
            let settled = self.settle(clock);
            strip |= settled;
            self.app.turn();
            let r = self.app.check();
            self.note(r);
            if !settled && self.app.requests().len() == self.seen {
                break;
            }
        }
        if let Some(next) = self.flights.iter().map(|f| f.due).reduce(f64::min) {
            tx.wake_at(next);
        }
        let structural = self.absorb();
        let strip_needs_view = strip && !self.write_strip();
        structural || strip_needs_view
    }

    fn issue(&mut self, clock: f64) -> bool {
        let requests = self.app.requests();
        let fresh = requests.len() > self.seen;
        for r in &requests[self.seen..] {
            let args: Vec<J> = { let h = self.host(); r.args.iter().map(|a| h.enc(a)).collect() };
            let reply = self.backend.call(&r.host, &args);
            let shown = args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ");
            self.calls.push(Call { host: r.host.clone(), args: shown, started: clock, settled: None });
            self.flights.push(Flight {
                due: clock + reply.after,
                host: r.host.clone(),
                number: r.number,
                result: reply.result,
                call: self.calls.len() - 1,
            });
        }
        self.seen = requests.len();
        fresh
    }

    fn settle(&mut self, clock: f64) -> bool {
        let mut due: Vec<Flight> = Vec::new();
        let mut i = 0;
        while i < self.flights.len() {
            if self.flights[i].due <= clock { due.push(self.flights.remove(i)) } else { i += 1 }
        }
        due.sort_by(|a, b| a.due.total_cmp(&b.due));
        let any = !due.is_empty();
        for f in due {
            let outcome = match &f.result {
                Ok(v) => summary(v),
                Err(e) => format!("failed: {e}"),
            };
            let started = self.calls[f.call].started;
            self.calls[f.call].settled = Some((f.due - started, outcome));
            let r = self.app.settle(&f.host, f.number, f.result.map(|v| json_value(&v)));
            self.note(r);
        }
        any
    }

    /// Takes the tree's writes. Children and props run the view; text goes into the committed
    /// elements in place.
    fn absorb(&mut self) -> bool {
        let writes = self.app.host.borrow_mut().take_writes();
        let Some(retained) = self.retained.borrow().clone() else { return !writes.is_empty() };
        let mut structural = false;
        let mut texts = Vec::new();
        for w in writes {
            match w {
                HostWrite::Children(_) => structural = true,
                HostWrite::Prop(n) if self.host().tag(n) == Some("input") => structural |= self.sync_input(n),
                HostWrite::Prop(_) => structural = true,
                HostWrite::Text(n) => texts.push(n),
            }
        }
        if structural {
            self.sync_inputs();
            return true;
        }
        texts.sort_unstable();
        texts.dedup();
        let h = self.host();
        for n in texts {
            let Some(p) = h.parent_of(n) else { continue };
            if text_only(&h, p) {
                retained.set_text(format!("l{p}"), joined(&h, p));
            } else {
                retained.set_text(format!("t{n}"), node_text(&h, n));
            }
        }
        false
    }

    /// A field whose `value` the plan set to something other than what was typed takes it.
    fn sync_input(&mut self, n: Node) -> bool {
        let value = prop_text(&self.host(), n, "value");
        match self.inputs.get_mut(&n) {
            Some(edit) if edit.text == value => false,
            Some(edit) => {
                *edit = TextEdit::new(value);
                true
            }
            None => true,
        }
    }

    fn sync_inputs(&mut self) {
        let mut found = Vec::new();
        inputs_under(&self.host(), 0, &mut found);
        self.inputs.retain(|n, _| found.contains(n));
        for n in found {
            self.edit(n);
        }
    }

    pub(crate) fn strip_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self.calls.iter().rev().take(STRIP).map(|c| {
            let at = format!("{:>6.2}s", c.started);
            match &c.settled {
                Some((took, outcome)) => format!("{at}  {}({})  {:.0} ms  {outcome}", c.host, c.args, took * 1000.0),
                None => format!("{at}  {}({})  …", c.host, c.args),
            }
        }).collect();
        lines.resize(STRIP, String::new());
        let landed = self.retained.borrow().as_ref().map_or(0, Retained::landed);
        lines.push(format!("host calls {} · view runs {} · text-only commits {landed}", self.calls.len(), self.views.get()));
        lines
    }

    /// False when there is no handle yet and the view has to draw the strip.
    fn write_strip(&self) -> bool {
        let Some(r) = self.retained.borrow().clone() else { return false };
        let mut shown = self.shown.borrow_mut();
        for (i, line) in self.strip_lines().into_iter().enumerate() {
            if shown[i] != line {
                r.set_text(format!("strip{i}"), line.clone());
                shown[i] = line;
            }
        }
        true
    }
}

fn mount(plan: &str) -> Result<App, String> {
    let plan: Plan = serde_json::from_str(plan).map_err(|e| format!("plan: {e}"))?;
    App::mount_unlogged(plan)
}

fn empty() -> App {
    let plan: Plan = serde_json::from_value(serde_json::json!({
        "version": 2, "component": "Empty", "hosts": [], "sources": [], "memos": [], "actions": [], "markup": []
    })).expect("an empty plan parses");
    App::mount_unlogged(plan).expect("an empty plan mounts")
}

fn summary(v: &J) -> String {
    match v {
        J::Array(xs) => format!("{} rows", xs.len()),
        J::Object(_) => "an object".into(),
        other => other.to_string(),
    }
}

pub(crate) fn prop_text(h: &TreeRenderer, n: Node, name: &str) -> String {
    h.prop(n, name).to_js_string().filter(|s| s != "undefined").unwrap_or_default()
}

pub(crate) fn node_text(h: &TreeRenderer, n: Node) -> String {
    h.view_node(n).1.to_js_string().unwrap_or_default()
}

/// An element whose children are all text renders them as one label.
pub(crate) fn text_only(h: &TreeRenderer, n: Node) -> bool {
    h.view_kids(n).all(|c| h.view_node(c).0.is_none())
}

pub(crate) fn joined(h: &TreeRenderer, n: Node) -> String {
    h.view_kids(n).map(|c| node_text(h, c)).collect()
}

fn inputs_under(h: &TreeRenderer, n: Node, out: &mut Vec<Node>) {
    for c in h.view_kids(n) {
        match h.view_node(c).0 {
            Some("input") => out.push(c),
            Some(_) => inputs_under(h, c, out),
            None => {}
        }
    }
}
