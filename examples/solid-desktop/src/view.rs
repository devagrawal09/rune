//! The plan's tree as Rune elements. Every host node keeps one key, so a text write lands in
//! the element the view built for it, and a row the plan rebuilds with the same `id` keeps its
//! place in the diff.

use rune::{Cx, El, ScrollSpec, Spring, TextStyle, Transition, col, text, text_input};
use rust_solid::renderer::TreeRenderer;

use crate::host::{Node, Solid, joined, node_text, prop_text, text_only};
use crate::look::{self, Look};

pub fn view(s: &Solid, cx: &mut Cx<Solid>) -> El {
    if s.retained.borrow().is_none() {
        *s.retained.borrow_mut() = Some(cx.retained());
    }
    s.views.set(s.views.get() + 1);
    let t = look::theme();
    let h = s.host();
    let base = look::root();
    let mut body = col().key("body").w_full().grow(1.0).min_h(0.0);
    for c in h.view_kids(h.root()) {
        body = body.child(node(s, &h, c, &base));
    }
    if let Some(e) = &s.error {
        body = body.child(text(e.clone()).key("fatal").wrap().font(TextStyle::new(12.0, 500.0))
            .color(rune::oklch(0.68, 0.17, 25.0)).px(24.0).py(8.0));
    }
    col().w_full().h_full().fill(t.bg).child(body).child(strip(s))
}

fn node(s: &Solid, h: &TreeRenderer, n: Node, parent: &Look) -> El {
    let Some(tag) = h.view_node(n).0 else {
        return label(node_text(h, n), parent).key(format!("t{n}"));
    };
    let class = prop_text(h, n, "class");
    let classes: Vec<&str> = class.split_whitespace().collect();
    let l = look::of(tag, &classes, parent);
    if tag == "input" {
        return field(s, h, n, &l);
    }
    let id = prop_text(h, n, "id");
    let key = if id.is_empty() { format!("k{n}") } else { format!("id:{id}") };
    let mut e = look::container(&l).key(key);
    if text_only(h, n) {
        if h.view_kids(n).next().is_some() {
            e = e.child(label(joined(h, n), &l).key(format!("l{n}")));
        }
    } else {
        for c in h.view_kids(n) {
            e = e.child(node(s, h, c, &l));
        }
    }
    if l.scroll {
        let spec = if l.follow { ScrollSpec::stick_bottom() } else { ScrollSpec::default() };
        e = e.scroll(spec).shrink().min_h(0.0);
    }
    if h.handles(n, "click") {
        if h.prop(n, "disabled").truthy() {
            e = e.opacity(0.4);
        } else {
            // The plan's writes decide what to draw: text lands in place, anything else asks
            // poll to run the view.
            e = e.on_press_tx(move |s: &mut Solid, tx| {
                s.click(n);
                tx.live_only();
                tx.request_poll();
            });
            if l.pressable {
                e = e.press_scale(0.97);
            }
        }
    }
    if l.enters {
        e = e.transition(Transition::opacity()).motion(Spring::new(0.35, 0.0));
    }
    e
}

fn label(content: String, l: &Look) -> El {
    let len = content.len();
    let mut e = text(content).font(l.font).color(l.color);
    if l.strike {
        e = e.strike(0, len, None);
    }
    if l.wrap { e.wrap() } else { e }
}

fn field(s: &Solid, h: &TreeRenderer, n: Node, l: &Look) -> El {
    let t = look::theme();
    let edit = s.inputs.get(&n).cloned().unwrap_or_else(|| rune::TextEdit::new(prop_text(h, n, "value")));
    let id = prop_text(h, n, "id");
    let key = if id.is_empty() { format!("in{n}") } else { format!("id:{id}") };
    let mut input = text_input(&edit, l.font).key(key).grow(1.0).color(l.color)
        .placeholder(prop_text(h, n, "placeholder"), t.text_secondary)
        .single_line()
        .on_edit(move |s: &mut Solid| s.edit(n));
    if h.handles(n, "input") {
        input = input.on_input_change(move |s: &mut Solid, _tx| {
            let value = s.edit(n).text.clone();
            s.dispatch(n, "input", Some(&value));
        });
    }
    if h.handles(n, "change") {
        input = input.on_submit(move |s: &mut Solid, _tx| s.dispatch(n, "change", None));
    }
    look::container(l).key(format!("k{n}")).child(input)
}

/// The host calls the plan made, and how often the view ran: what makes the async visible.
fn strip(s: &Solid) -> El {
    let t = look::theme();
    let lines = s.strip_lines();
    *s.shown.borrow_mut() = lines.clone();
    let mut c = col().key("strip").w_full().px(16.0).py(10.0).gap(3.0).fill(t.card)
        .border_sides([1.0, 0.0, 0.0, 0.0], t.card_border);
    let last = lines.len() - 1;
    for (i, line) in lines.into_iter().enumerate() {
        let style = TextStyle::new(11.0, 500.0).mono();
        let color = if i == last { look::accent() } else { t.text_secondary };
        c = c.child(text(line).key(format!("strip{i}")).font(style).color(color).single_line());
    }
    c
}
