import { action, affects, createMemo, createOptimistic, createSignal, Errored, For, isPending, Loading, refresh, Show } from "solid-js";
import { loadIssue, postComment, searchIssues, setOpen } from "./api";

export default function Issues() {
  const [query, setQuery] = createSignal("");
  const [state, setState] = createSignal("open");
  const [selected, setSelected] = createSignal(0);
  const [draft, setDraft] = createSignal("");
  const [status, setStatus] = createSignal("");
  const found = createMemo(() => searchIssues(query(), state()));
  const [shown, setShown] = createOptimistic(() => found());
  const issue = createMemo(() => loadIssue(selected()));
  const toggle = action(function* (id: number, open: boolean) {
    setShown((list) => list.filter((i) => i.id !== id));
    try {
      const ok: boolean = yield setOpen(id, open);
      refresh(found);
      refresh(issue);
      setStatus(open ? "Reopened #" + id : "Closed #" + id);
    } catch (error) {
      setStatus("Could not update #" + id);
    }
  });
  const comment = action(function* () {
    affects(issue);
    try {
      const n: number = yield postComment(selected(), draft());
      setDraft("");
      refresh(issue);
      refresh(found);
    } catch (error) {
      setStatus("Could not post that");
    }
  });
  return (
    <main class="app">
      <div class="bar">
        <h1 class="title">Issues</h1>
        <span class="spacer"></span>
        <p class="status">{status()}</p>
      </div>
      <div class="split">
        <div class="side">
          <div class="composer">
            <input id="search" class="field" value={query()} placeholder="Search issues" onInput={(e: { target: { value: string } }) => setQuery(e.target.value)} />
          </div>
          <div class="bar">
            <div class="tabs">
              <button id="tab-open" class={state() === "open" ? "tab on" : "tab"} onClick={() => setState("open")}>Open</button>
              <button id="tab-closed" class={state() === "closed" ? "tab on" : "tab"} onClick={() => setState("closed")}>Closed</button>
            </div>
            <span class="spacer"></span>
            <small>{"" + (isPending(() => found()) ? "Searching…" : "")}</small>
          </div>
          <Errored fallback={(err) => <p class="error">Search failed <button class="ghost" onClick={() => refresh(found)}>Retry</button></p>}>
            <Loading fallback={<p class="muted">Loading issues…</p>}>
              <Show when={shown().length > 0} fallback={<p class="muted">No issues match</p>}>
                <div class="list">
                  <For each={shown()}>{(i) => (
                    <div id={"issue-" + i.id} class={i.id === selected() ? "item selected" : "item"} onClick={() => setSelected(i.id)}>
                      <span class="label">{i.title}</span>
                      <div class="meta">
                        <span>{"#" + i.id + "  ·  " + i.label + "  ·  " + i.comments + " comments"}</span>
                      </div>
                    </div>
                  )}</For>
                </div>
              </Show>
            </Loading>
          </Errored>
        </div>
        <Show when={selected() !== 0} fallback={<div class="detail"><p class="muted">Pick an issue on the left.</p></div>}>
          <Errored fallback={(err) => <div class="detail"><p class="error">Could not load this issue <button class="ghost" onClick={() => refresh(issue)}>Retry</button></p></div>}>
            <Loading fallback={<div class="detail"><p class="muted">Loading…</p></div>}>
              <div class={isPending(() => issue()) ? "detail stale" : "detail"}>
                <div class="bar">
                  <span class={issue().open ? "badge open" : "badge closed"}>{"" + (issue().open ? "Open" : "Closed")}</span>
                  <small>{"#" + issue().id + " by " + issue().author}</small>
                  <span class="spacer"></span>
                  <Show when={issue().open} fallback={<button id="reopen" class="ghost" onClick={() => toggle(selected(), true)}>Reopen</button>}>
                    <button id="close" class="ghost" onClick={() => toggle(selected(), false)}>Close issue</button>
                  </Show>
                </div>
                <h2>{issue().title}</h2>
                <p class="body">{issue().body}</p>
                <div class="comments thread">
                  <For each={issue().comments}>{(c) => (
                    <div id={"comment-" + c.id} class="comment">
                      <span class="author">{c.author}</span>
                      <span>{c.body}</span>
                    </div>
                  )}</For>
                </div>
                <div class="composer">
                  <input id="comment" class="field" value={draft()} placeholder="Leave a comment" onInput={(e: { target: { value: string } }) => setDraft(e.target.value)} onChange={() => comment()} />
                  <button id="post" class="primary" disabled={draft() === ""} onClick={() => comment()}>{"" + (isPending(() => issue()) ? "Posting…" : "Comment")}</button>
                </div>
              </div>
            </Loading>
          </Errored>
        </Show>
      </div>
    </main>
  );
}
