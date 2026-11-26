//! Desktop apps written in Solid TSX: the frontend compiles each to a plan, rust-solid runs
//! the plan's reactive graph, and Rune draws the tree it builds. A text change writes into the
//! committed element without running the view; inserts, removals and prop changes run it.

mod host;
mod look;
mod view;

pub use host::{Backend, Reply, Solid};

use rune::{AppSpec, Tx};

fn poll(s: &mut Solid, clock: f64, tx: &mut Tx) -> bool {
    s.poll(clock, tx)
}

/// An app for `rune::run`: the window draws the plan `init` mounts.
pub fn app(title: &'static str, size: (f32, f32), init: fn() -> Solid) -> AppSpec<Solid> {
    AppSpec { poll: Some(poll), ..AppSpec::new(title, size, init, view::view) }
}
