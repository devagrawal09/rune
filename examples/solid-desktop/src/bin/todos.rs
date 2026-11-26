//! Todos: a text field, filters, and toggles and removals that show at once and are undone
//! when the save fails. `cargo run -p solid-desktop --bin todos`.

#![forbid(unsafe_code)]

use serde_json::{Value as J, json};
use solid_desktop::{Backend, Reply, Solid};

struct Todo {
    id: i64,
    title: String,
    done: bool,
}

/// An in-memory list behind a slow network. Every fourth save fails.
struct Todos {
    list: Vec<Todo>,
    next: i64,
    saves: u32,
}

impl Default for Todos {
    fn default() -> Self {
        let titles = ["Pick a name for the release", "Record the demo", "Write the changelog"];
        let list = titles.iter().zip(1..).map(|(t, id)| Todo { id, title: t.to_string(), done: id == 1 }).collect();
        Todos { list, next: 4, saves: 0 }
    }
}

impl Backend for Todos {
    fn call(&mut self, host: &str, args: &[J]) -> Reply {
        match host {
            "loadTodos" => {
                let rows: Vec<J> = self.list.iter().map(|t| json!({"id": t.id, "title": t.title, "done": t.done})).collect();
                Reply::ok(0.45, J::Array(rows))
            }
            "addTodo" => {
                let title = args.first().and_then(J::as_str).unwrap_or_default().to_string();
                self.list.push(Todo { id: self.next, title, done: false });
                self.next += 1;
                Reply::ok(0.6, json!(self.next - 1))
            }
            "setDone" | "removeTodo" => {
                self.saves += 1;
                if self.saves % 4 == 0 {
                    return Reply::fail(0.7, "the server said no");
                }
                let id = args.first().and_then(J::as_i64).unwrap_or_default();
                if host == "setDone" {
                    let done = args.get(1).and_then(J::as_bool).unwrap_or_default();
                    if let Some(t) = self.list.iter_mut().find(|t| t.id == id) {
                        t.done = done;
                    }
                } else {
                    self.list.retain(|t| t.id != id);
                }
                Reply::ok(0.7, json!(true))
            }
            other => Reply::fail(0.0, format!("no host function {other}")),
        }
    }
}

fn init() -> Solid {
    Solid::new(include_str!("../../plans/todos.plan.json"), Box::new(Todos::default()))
}

fn main() {
    rune::run(solid_desktop::app("Todos", (520.0, 560.0), init));
}
