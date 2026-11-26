//! Issues: a search that asks the server as you type, a list you pick from, and a detail pane
//! that keeps the last issue on screen, dimmed, while the next one loads.
//! `cargo run -p solid-desktop --bin issues`.

#![forbid(unsafe_code)]

use serde_json::{Value as J, json};
use solid_desktop::{Backend, Reply, Solid};

struct Issue {
    id: i64,
    title: &'static str,
    label: &'static str,
    body: &'static str,
    author: &'static str,
    open: bool,
    comments: Vec<(i64, String, String)>,
}

struct Tracker {
    issues: Vec<Issue>,
    next_comment: i64,
}

fn issue(id: i64, title: &'static str, label: &'static str, author: &'static str, body: &'static str, open: bool) -> Issue {
    Issue { id, title, label, body, author, open, comments: Vec::new() }
}

impl Default for Tracker {
    fn default() -> Self {
        let mut issues = vec![
            issue(101, "Scroll position resets after a refresh", "bug", "maya", "Refreshing the list scrolls it back to the top. It should keep the row under the pointer where it was.", true),
            issue(102, "Keyboard shortcut to focus search", "feature", "jon", "Cmd+K should put the caret in the search field from anywhere in the window.", true),
            issue(103, "Dark mode contrast on badges", "design", "ana", "Closed badges are hard to read on the card colour in dark mode.", true),
            issue(104, "Crash when the network drops mid-save", "bug", "lee", "Pulling the cable while a comment posts leaves the composer stuck on Posting.", true),
            issue(105, "Remember the last open filter", "feature", "maya", "Coming back to the app should show the filter that was open when it closed.", true),
            issue(106, "Typo in the onboarding sheet", "docs", "sam", "\"Recieve\" on the second page.", false),
            issue(107, "Slow first load on large projects", "perf", "jon", "Projects with thousands of issues take seconds to show anything.", false),
        ];
        issues[0].comments.push((1, "jon".into(), "I can reproduce this on every refresh.".into()));
        issues[0].comments.push((2, "maya".into(), "Same here, with and without a filter.".into()));
        issues[3].comments.push((3, "ana".into(), "Retrying the save should be enough here.".into()));
        Tracker { issues, next_comment: 4 }
    }
}

impl Backend for Tracker {
    fn call(&mut self, host: &str, args: &[J]) -> Reply {
        let num = |i: usize| args.get(i).and_then(J::as_i64).unwrap_or_default();
        let text = |i: usize| args.get(i).and_then(J::as_str).unwrap_or_default().to_string();
        match host {
            "searchIssues" => {
                let (query, state) = (text(0).to_lowercase(), text(1));
                let rows: Vec<J> = self.issues.iter()
                    .filter(|i| i.open == (state == "open") && i.title.to_lowercase().contains(&query))
                    .map(|i| json!({"id": i.id, "title": i.title, "label": i.label, "open": i.open, "comments": i.comments.len()}))
                    .collect();
                // A longer query is a narrower search: the server answers it sooner.
                Reply::ok(0.9 - 0.1 * query.len().min(5) as f64, J::Array(rows))
            }
            "loadIssue" => {
                let id = num(0);
                let Some(i) = self.issues.iter().find(|i| i.id == id) else {
                    return Reply::ok(0.1, json!({"id": 0, "title": "", "body": "", "open": false, "author": "", "comments": []}));
                };
                let comments: Vec<J> = i.comments.iter().map(|(id, a, b)| json!({"id": id, "author": a, "body": b})).collect();
                // Issue 104 is on a server that is down.
                if id == 104 {
                    return Reply::fail(0.8, "503 from the issue service");
                }
                Reply::ok(0.7, json!({"id": i.id, "title": i.title, "body": i.body, "open": i.open, "author": i.author, "comments": comments}))
            }
            "setOpen" => {
                let (id, open) = (num(0), args.get(1).and_then(J::as_bool).unwrap_or_default());
                if let Some(i) = self.issues.iter_mut().find(|i| i.id == id) {
                    i.open = open;
                }
                Reply::ok(0.6, json!(true))
            }
            "postComment" => {
                let (id, body) = (num(0), text(1));
                let n = self.next_comment;
                self.next_comment += 1;
                if let Some(i) = self.issues.iter_mut().find(|i| i.id == id) {
                    i.comments.push((n, "you".into(), body));
                }
                Reply::ok(0.6, json!(n))
            }
            other => Reply::fail(0.0, format!("no host function {other}")),
        }
    }
}

fn init() -> Solid {
    Solid::new(include_str!("../../plans/issues.plan.json"), Box::new(Tracker::default()))
}

fn main() {
    rune::run(solid_desktop::app("Issues", (900.0, 620.0), init));
}
