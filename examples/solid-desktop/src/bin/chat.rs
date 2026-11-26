//! Chat: a conversation that loads, a message that shows as you send it, a reply that arrives
//! a moment later, and a send that fails and offers the text back. `cargo run -p solid-desktop --bin chat`.

#![forbid(unsafe_code)]

use serde_json::{Value as J, json};
use solid_desktop::{Backend, Reply, Solid};

struct Line {
    id: i64,
    mine: bool,
    text: String,
}

/// A conversation with Ada behind a slow network. The third send fails.
struct Chat {
    lines: Vec<Line>,
    sends: u32,
    replies: usize,
}

const OPENING: [(bool, &str); 4] = [
    (false, "Morning! Did the desktop build come together?"),
    (true, "It did. Solid drives the tree, Rune draws it."),
    (false, "Nice. How does typing feel?"),
    (true, "Like any other field. Want to try it?"),
];

const REPLIES: [&str; 3] = [
    "Ha, that was quick. The bubble showed before the server answered.",
    "And the typing line went away by itself when my reply landed.",
    "Send another one, I'm watching the strip at the bottom.",
];

impl Default for Chat {
    fn default() -> Self {
        let lines = OPENING.iter().zip(1..).map(|(&(mine, text), id)| Line { id, mine, text: text.into() }).collect();
        Chat { lines, sends: 0, replies: 0 }
    }
}

impl Chat {
    fn push(&mut self, mine: bool, text: String) -> i64 {
        let id = self.lines.len() as i64 + 1;
        self.lines.push(Line { id, mine, text });
        id
    }
}

impl Backend for Chat {
    fn call(&mut self, host: &str, args: &[J]) -> Reply {
        match host {
            "loadMessages" => {
                let rows: Vec<J> = self.lines.iter().map(|l| {
                    json!({"id": l.id, "author": if l.mine { "me" } else { "Ada" }, "text": l.text, "mine": l.mine})
                }).collect();
                Reply::ok(0.5, J::Array(rows))
            }
            "sendMessage" => {
                self.sends += 1;
                if self.sends == 3 {
                    return Reply::fail(0.9, "no connection");
                }
                let text = args.first().and_then(J::as_str).unwrap_or_default().to_string();
                Reply::ok(0.6, json!(self.push(true, text)))
            }
            "awaitReply" => {
                let text = REPLIES[self.replies % REPLIES.len()].to_string();
                self.replies += 1;
                self.push(false, text);
                Reply::ok(1.6, json!(true))
            }
            other => Reply::fail(0.0, format!("no host function {other}")),
        }
    }
}

fn init() -> Solid {
    Solid::new(include_str!("../../plans/chat.plan.json"), Box::new(Chat::default()))
}

fn main() {
    rune::run(solid_desktop::app("Chat", (520.0, 620.0), init));
}
