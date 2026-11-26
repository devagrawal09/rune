//! The apps' stylesheet: what a tag and its classes look like in Rune. Text styles flow down
//! to descendants, as CSS inherits `color` and `font`.

use rune::tok::{self, Theme};
use rune::{Color, El, TextStyle, col, row};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Row,
    Col,
}

#[derive(Clone, Copy)]
pub struct Look {
    pub dir: Dir,
    pub fill: Option<Color>,
    pub border: Option<Color>,
    pub radius: f32,
    pub pad: (f32, f32),
    pub gap: f32,
    pub h: Option<f32>,
    pub w: Option<f32>,
    pub grow: bool,
    pub wide: bool,
    pub tall: bool,
    pub center: bool,
    pub between: bool,
    pub end: bool,
    pub scroll: bool,
    /// A scroll that follows its bottom (a chat thread).
    pub follow: bool,
    pub hover: Option<Color>,
    pub pressable: bool,
    /// Rows that come and go fade and slide; their siblings spring to new places.
    pub enters: bool,
    pub opacity: f32,
    pub font: TextStyle,
    pub color: Color,
    pub strike: bool,
    pub wrap: bool,
}

pub fn theme() -> Theme {
    tok::theme(true)
}

pub fn accent() -> Color {
    tok::accent(0, true)
}

fn green() -> Color {
    tok::accent(4, true)
}

fn red() -> Color {
    rune::oklch(0.68, 0.17, 25.0)
}

fn amber() -> Color {
    tok::accent(3, true)
}

/// The window itself.
pub fn root() -> Look {
    let t = theme();
    Look {
        dir: Dir::Col,
        fill: None,
        border: None,
        radius: 0.0,
        pad: (0.0, 0.0),
        gap: 0.0,
        h: None,
        w: None,
        grow: false,
        wide: false,
        tall: false,
        center: false,
        between: false,
        end: false,
        scroll: false,
        follow: false,
        hover: None,
        pressable: false,
        enters: false,
        opacity: 1.0,
        font: TextStyle::new(13.0, 400.0),
        color: t.text,
        strike: false,
        wrap: false,
    }
}

/// `tag` with `classes` under `parent`: block layout from the tag, text inherited.
pub fn of(tag: &str, classes: &[&str], parent: &Look) -> Look {
    let t = theme();
    let mut l = Look { dir: Dir::Col, ..root() };
    (l.font, l.color, l.strike) = (parent.font, parent.color, parent.strike);
    match tag {
        "span" | "label" => l.dir = Dir::Row,
        "button" => (l.dir, l.strike) = (Dir::Row, false),
        "li" => l.dir = Dir::Row,
        "h1" => l.font = TextStyle::new(22.0, 700.0),
        "h2" => l.font = TextStyle::new(15.0, 600.0),
        "small" => (l.font, l.color) = (TextStyle::new(11.0, 500.0), t.text_secondary),
        _ => {}
    }
    for c in classes {
        apply(&mut l, c, &t);
    }
    l
}

fn apply(l: &mut Look, class: &str, t: &Theme) {
    match class {
        "app" => (l.wide, l.tall, l.pad, l.gap) = (true, true, (24.0, 22.0), 14.0),
        "title" => l.font = TextStyle::new(22.0, 700.0),
        "subtitle" => (l.font, l.color) = (TextStyle::new(12.0, 500.0), t.text_secondary),
        "bar" => (l.dir, l.wide, l.gap, l.center) = (Dir::Row, true, 8.0, true),
        "composer" => (l.dir, l.wide, l.gap, l.center) = (Dir::Row, true, 8.0, true),
        "field" => {
            (l.dir, l.grow, l.h, l.pad, l.radius) = (Dir::Row, true, Some(34.0), (12.0, 0.0), 8.0);
            (l.fill, l.border, l.center) = (Some(t.card), Some(t.card_border), true);
        }
        "primary" => {
            (l.h, l.pad, l.radius, l.center, l.pressable) = (Some(34.0), (16.0, 0.0), 8.0, true, true);
            (l.fill, l.color, l.font) = (Some(accent()), Color::WHITE, TextStyle::new(13.0, 600.0));
        }
        "ghost" => {
            (l.h, l.pad, l.radius, l.center, l.pressable) = (Some(26.0), (8.0, 0.0), 6.0, true, true);
            (l.hover, l.color, l.font) = (Some(t.hover), t.text_secondary, TextStyle::new(12.0, 500.0));
        }
        "tabs" => {
            (l.dir, l.gap, l.pad, l.radius, l.fill) = (Dir::Row, 2.0, (3.0, 3.0), 9.0, Some(t.control_off));
        }
        "tab" => {
            (l.h, l.pad, l.radius, l.center, l.pressable) = (Some(26.0), (14.0, 0.0), 7.0, true, true);
            (l.color, l.font, l.hover) = (t.text_secondary, TextStyle::new(12.0, 500.0), Some(t.hover));
        }
        "on" => (l.fill, l.color, l.hover) = (Some(t.card), t.text, None),
        "list" => (l.wide, l.gap, l.scroll, l.grow) = (true, 4.0, true, true),
        "row" => {
            (l.wide, l.h, l.pad, l.gap, l.center) = (true, Some(40.0), (10.0, 0.0), 10.0, true);
            (l.radius, l.fill, l.enters) = (8.0, Some(t.card), true);
        }
        "done" => (l.color, l.strike) = (t.text_secondary, true),
        "check" => {
            (l.w, l.h, l.radius, l.center, l.pressable) = (Some(20.0), Some(20.0), 10.0, true, true);
            (l.border, l.color, l.font) = (Some(t.text_secondary), Color::WHITE, TextStyle::new(12.0, 700.0));
            l.strike = false;
        }
        "checked" => (l.fill, l.border) = (Some(green()), Some(green())),
        "label" => l.grow = true,
        "footer" => (l.font, l.color) = (TextStyle::new(12.0, 500.0), t.text_secondary),
        "status" => (l.font, l.color) = (TextStyle::new(12.0, 500.0), accent()),
        "muted" => l.color = t.text_secondary,
        "error" => (l.dir, l.gap, l.center, l.color) = (Dir::Row, 8.0, true, red()),
        "pending" => l.opacity = 0.55,
        "spacer" => l.grow = true,
        "split" => (l.dir, l.wide, l.grow, l.gap) = (Dir::Row, true, true, 16.0),
        "side" => (l.w, l.gap, l.tall) = (Some(300.0), 8.0, true),
        "detail" => {
            (l.grow, l.tall, l.gap, l.pad, l.radius) = (true, true, 10.0, (18.0, 16.0), 12.0);
            (l.fill, l.border) = (Some(t.card), Some(t.card_border));
        }
        "item" => {
            (l.wide, l.pad, l.gap, l.radius, l.pressable) = (true, (12.0, 9.0), 3.0, 8.0, true);
            (l.hover, l.enters) = (Some(t.hover), true);
        }
        "selected" => (l.fill, l.hover) = (Some(accent().alpha(0.22)), None),
        "meta" => (l.dir, l.gap, l.center, l.font, l.color) = (Dir::Row, 6.0, true, TextStyle::new(11.5, 500.0), t.text_secondary),
        "badge" => (l.pad, l.radius, l.h, l.center, l.font) = ((7.0, 0.0), 9.0, Some(18.0), true, TextStyle::new(11.0, 600.0)),
        "open" => (l.fill, l.color) = (Some(green().alpha(0.2)), green()),
        "closed" => (l.fill, l.color) = (Some(t.control_off), t.text_secondary),
        "body" => (l.wrap, l.color) = (true, t.text),
        "comments" => (l.gap, l.wide) = (8.0, true),
        "comment" => {
            (l.wide, l.pad, l.gap, l.radius, l.fill, l.enters) = (true, (12.0, 9.0), 3.0, 8.0, Some(t.bg), true);
            l.wrap = true;
        }
        "author" => (l.font, l.color) = (TextStyle::new(11.5, 600.0), t.text_secondary),
        "thread" => (l.wide, l.grow, l.gap, l.scroll, l.follow) = (true, true, 8.0, true, true),
        "bubble" => {
            (l.pad, l.radius, l.gap, l.enters, l.wrap) = ((12.0, 8.0), 14.0, 2.0, true, true);
            l.fill = Some(t.card);
        }
        "me" => (l.fill, l.color, l.end) = (Some(accent()), Color::WHITE, true),
        "them" => {}
        "typing" => (l.color, l.font) = (t.text_secondary, TextStyle::new(12.0, 500.0).italic()),
        "line" => (l.dir, l.wide) = (Dir::Row, true),
        "mine" => (l.dir, l.wide, l.end) = (Dir::Row, true, true),
        "cities" => (l.dir, l.gap, l.wide) = (Dir::Row, 6.0, true),
        "hero" => {
            (l.wide, l.gap, l.pad, l.radius, l.fill) = (true, 4.0, (20.0, 18.0), 14.0, Some(t.card));
            l.border = Some(t.card_border);
        }
        "big" => l.font = TextStyle::new(40.0, 600.0),
        "days" => (l.wide, l.gap) = (true, 4.0),
        "day" => (l.dir, l.wide, l.h, l.gap, l.center, l.enters) = (Dir::Row, true, Some(30.0), 12.0, true, true),
        "name" => (l.w, l.color) = (Some(44.0), t.text_secondary),
        "low" => (l.w, l.color) = (Some(36.0), t.text_secondary),
        "high" => l.w = Some(36.0),
        "hot" => l.color = amber(),
        "stale" => l.opacity = 0.5,
        _ => {}
    }
}

/// An empty box laid out as `l` says.
pub fn container(l: &Look) -> El {
    let mut e = match l.dir {
        Dir::Row => row(),
        Dir::Col => col(),
    };
    if l.wide { e = e.w_full() }
    if l.tall { e = e.h_full() }
    if l.grow { e = e.grow(1.0) }
    if let Some(w) = l.w { e = e.w(w) }
    if let Some(h) = l.h { e = e.h(h) }
    if l.pad != (0.0, 0.0) { e = e.px(l.pad.0).py(l.pad.1) }
    if l.gap > 0.0 { e = e.gap(l.gap) }
    if l.radius > 0.0 { e = e.radius(l.radius) }
    if let Some(f) = l.fill { e = e.fill(f) }
    if let Some(b) = l.border { e = e.border(1.0, b) }
    if l.center && l.dir == Dir::Row { e = e.center_y() }
    if l.center && l.w.is_some() && l.dir == Dir::Row { e = e.justify_center() }
    if l.between { e = e.justify_between() }
    if l.end { e = e.justify_end() }
    if let Some(h) = l.hover { e = e.hover(h) }
    if l.opacity < 1.0 { e = e.opacity(l.opacity) }
    e
}
