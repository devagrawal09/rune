// jfb's keyed table as a rust-solid plan v2 program, and the store writes of
// its header buttons as an app's handlers make them. Shared by the rust-solid
// bench and the Rune glue bench.
#![allow(dead_code)]
use crate::common::Row;
use rust_solid::plan_v2::{App, Plan};
use rust_solid::slice1::{JsObj, Key, Value};
use serde_json::{json, Value as J};
use std::rc::Rc;

fn lit(v: J) -> J { json!({"op":"literal","value":v}) }
fn read(s: &str) -> J { json!({"op":"read","source":s}) }
fn local(s: &str) -> J { json!({"op":"local","name":s}) }
fn get(o: J, k: &str) -> J { json!({"op":"get","object":o,"key":k}) }
fn text(v: J) -> J { json!({"kind":"text","value":v}) }
fn el(tag: &str, events: J, children: Vec<J>) -> J { json!({"kind":"element","tag":tag,"events":events,"children":children}) }
fn click(steps: J) -> J { json!([{"name":"onClick","steps":steps}]) }

/// The row as jfb's Solid app writes it, the class as a text cell.
pub fn row() -> J {
    let id = get(local("r"), "id");
    let marker = json!({"op":"cond","test":{"op":"eq","left":id,"right":read("sel")},"then":lit(json!("danger")),"else":lit(json!("-"))});
    let select = click(json!([{"op":"write","source":"sel","value":get(local("r"),"id")}]));
    let others = json!({"op":"filter","array":get(read("rows"),"list"),"param":"x","body":
        {"op":"ne","left":get(local("x"),"id"),"right":get(local("r"),"id")}});
    let remove = click(json!([{"op":"write","source":"rows","path":[{"key":"list"}],"value":others}]));
    el("tr", json!([]), vec![
        // Its own cell: a text beside siblings is replaced, not edited, when it changes.
        el("td", json!([]), vec![text(marker)]),
        el("td", json!([]), vec![text(get(local("r"), "id"))]),
        el("td", json!([]), vec![el("a", select, vec![text(get(local("r"), "label"))])]),
        el("td", json!([]), vec![el("a", remove, vec![el("span", json!([]), vec![])])]),
        el("td", json!([]), vec![]),
    ])
}

pub fn sources() -> Vec<J> {
    vec![json!({"name":"rows","kind":"store","seed":{"op":"object","fields":[{"key":"list","value":{"op":"array","items":[]}}]}}),
         json!({"name":"sel","kind":"signal","seed":lit(json!(-1))})]
}

pub fn plan() -> Plan {
    let tbody = el("tbody", json!([]), vec![json!({"kind":"for","each":get(read("rows"),"list"),"item":"r","key":"identity","children":[row()]})]);
    serde_json::from_value(json!({"version":2,"component":"Main","hosts":[],"sources":sources(),"memos":[],"actions":[],"markup":[tbody]})).unwrap()
}

fn obj(r: &Row) -> Value {
    Value::Obj(JsObj::new_object(vec![(Rc::from("id"), Value::Int(r.id as i64)), (Rc::from("label"), Value::str(&r.label))]))
}
fn int(v: Value) -> i64 { match v { Value::Int(n) => n, o => panic!("not an int: {o:?}") } }

/// The store writes of jfb's header buttons, for any mounted App.
pub struct Ops { pub next: u64 }
impl Ops {
    pub fn set_rows(&mut self, app: &mut App, n: usize) {
        let rows = crate::common::build(&mut self.next, n);
        app.store_set("rows", &mut |rt, d| {
            let list = Value::Obj(JsObj::new_array(rows.iter().map(obj).collect()));
            rt.js_set(&d, &Key::name("list"), list)
        }).unwrap();
    }
    pub fn add(&mut self, app: &mut App) {
        let rows = crate::common::build(&mut self.next, 1000);
        app.store_set("rows", &mut |rt, d| {
            let list = rt.js_get(&d, &Key::name("list"))?;
            let len = int(rt.js_get(&list, &Key::name("length"))?) as u32;
            for (i, r) in rows.iter().enumerate() { rt.js_set(&list, &Key::Idx(len + i as u32), obj(r))?; }
            Ok(())
        }).unwrap();
    }
    pub fn update(&mut self, app: &mut App) {
        app.store_set("rows", &mut |rt, d| {
            let list = rt.js_get(&d, &Key::name("list"))?;
            let len = int(rt.js_get(&list, &Key::name("length"))?) as u32;
            for i in (0..len).step_by(10) {
                let r = rt.js_get(&list, &Key::Idx(i))?;
                let l = rt.js_get(&r, &Key::name("label"))?.to_js_string().unwrap_or_default();
                rt.js_set(&r, &Key::name("label"), Value::str(&(l + " !!!")))?;
            }
            Ok(())
        }).unwrap();
    }
    pub fn clear(&mut self, app: &mut App) {
        app.store_set("rows", &mut |rt, d| rt.js_set(&d, &Key::name("list"), Value::Obj(JsObj::new_array(vec![])))).unwrap();
    }
    pub fn swap(&mut self, app: &mut App) {
        app.store_set("rows", &mut |rt, d| {
            let list = rt.js_get(&d, &Key::name("list"))?;
            if int(rt.js_get(&list, &Key::name("length"))?) > 998 {
                let a = rt.js_get(&list, &Key::Idx(1))?;
                let b = rt.js_get(&list, &Key::Idx(998))?;
                rt.js_set(&list, &Key::Idx(1), b)?;
                rt.js_set(&list, &Key::Idx(998), a)?;
            }
            Ok(())
        }).unwrap();
    }
}

/// The rows under the host's tbody as (marker, id, label), and a row's link.
pub fn table(app: &App, tbody: usize) -> Vec<(String, String, String)> {
    let h = app.host.borrow();
    h.children(tbody).into_iter().filter(|&tr| h.tag(tr) == Some("tr")).map(|tr| {
        let k = h.children(tr);
        (h.frame(k[0]), h.frame(k[1]), h.frame(k[2]))
    }).collect()
}
pub fn link(app: &App, tbody: usize, row: usize, cell: usize) -> usize {
    let h = app.host.borrow();
    let tr = h.children(tbody).into_iter().filter(|&tr| h.tag(tr) == Some("tr")).nth(row).expect("row");
    h.children(h.children(tr)[cell])[0]
}


/// The windowed program for the glue: the rows sliced to [winStart, winEnd)
/// inside a "vlist", with a `count` memo for the list's full length.
pub fn plan_lazy() -> Plan {
    let each = json!({"op":"slice","array":get(read("rows"),"list"),"start":read("winStart"),"end":read("winEnd")});
    let tbody = el("vlist", json!([]), vec![json!({"kind":"for","each":each,"item":"r","key":"identity","children":[row()]})]);
    let mut s = sources();
    s.push(json!({"name":"winStart","kind":"signal","seed":lit(json!(0))}));
    s.push(json!({"name":"winEnd","kind":"signal","seed":lit(json!(0))}));
    let memos = json!([{"name":"count","value":{"op":"length","array":get(read("rows"),"list")}}]);
    serde_json::from_value(json!({"version":2,"component":"Main","hosts":[],"sources":s,"memos":memos,"actions":[],"markup":[tbody]})).unwrap()
}

/// The mount's table body ("tbody", or "vlist" in the windowed program).
pub fn body(app: &App) -> usize {
    let h = app.host.borrow();
    h.children(h.root()).into_iter().find(|&n| matches!(h.tag(n), Some("tbody" | "vlist"))).expect("table body")
}
