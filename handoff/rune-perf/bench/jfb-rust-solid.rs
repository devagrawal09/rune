// jfb keyed on rust-solid alone: plan v2 for the view and the row clicks, host
// code for the header buttons, writing the store as an app's handler would
// (`setStore(s => { s.list = buildData(1000) })`). The recording host keeps
// the tree; there is no layout. Timed region: the write, then one turn.
// Run from rust-solid/: cargo run --release --example jfb -- [op|all] [reps]
#[path = "jfb_common.rs"]
mod common;
use common::{Driver, Shown};
#[path = "jfb_solid_plan.rs"]
mod solid;
use rust_solid::plan_v2::App;
use solid::{link, plan, table, Ops};

struct Solid { app: App, ops: Ops, tbody: usize }

impl Driver for Solid {
    fn run(&mut self) { self.ops.set_rows(&mut self.app, 1000); self.app.turn() }
    fn runlots(&mut self) { self.ops.set_rows(&mut self.app, 10_000); self.app.turn() }
    fn add(&mut self) { self.ops.add(&mut self.app); self.app.turn() }
    fn update(&mut self) { self.ops.update(&mut self.app); self.app.turn() }
    fn clear(&mut self) { self.ops.clear(&mut self.app); self.app.turn() }
    fn swap(&mut self) { self.ops.swap(&mut self.app); self.app.turn() }
    fn select(&mut self, row: usize) { let a = link(&self.app, self.tbody, row, 2); self.app.click(a).unwrap(); self.app.turn() }
    fn remove(&mut self, row: usize) { let a = link(&self.app, self.tbody, row, 3); self.app.click(a).unwrap(); self.app.turn() }
    fn shown(&mut self) -> Shown { self.app.check().unwrap(); Shown { rows: table(&self.app, self.tbody), windowed: false } }
}

impl Drop for Solid { fn drop(&mut self) { self.app.dispose() } }

fn main() {
    common::main_with("rust-solid", || {
        let mut app = App::mount_unlogged(plan()).unwrap();
        app.turn();
        let tbody = solid::body(&app);
        Solid { app, ops: Ops { next: 1 }, tbody }
    });
}
