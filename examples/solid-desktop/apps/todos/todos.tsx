import { action, affects, createMemo, createOptimistic, createSignal, Errored, For, isPending, Loading, refresh, Show } from "solid-js";
import { addTodo, loadTodos, removeTodo, setDone } from "./api";

export default function Todos() {
  const todos = createMemo(() => loadTodos());
  const [shown, setShown] = createOptimistic(() => todos());
  const [draft, setDraft] = createSignal("");
  const [view, setView] = createSignal("all");
  const [status, setStatus] = createSignal("");
  const visible = createMemo(() => shown().filter((t) => view() === "all" || (view() === "done" ? t.done : !t.done)));
  const left = createMemo(() => shown().filter((t) => !t.done).length);
  const add = action(function* () {
    affects(todos);
    try {
      const id: number = yield addTodo(draft());
      setDraft("");
      refresh(todos);
      setStatus("Added #" + id);
    } catch (error) {
      setStatus("Could not add that");
    }
  });
  const toggle = action(function* (id: number, done: boolean) {
    setShown((list) => list.map((t) => (t.id === id ? { id: t.id, title: t.title, done: done } : t)));
    try {
      const ok: boolean = yield setDone(id, done);
      refresh(todos);
    } catch (error) {
      setStatus("Could not save #" + id);
    }
  });
  const remove = action(function* (id: number) {
    setShown((list) => list.filter((t) => t.id !== id));
    try {
      const ok: boolean = yield removeTodo(id);
      refresh(todos);
    } catch (error) {
      setStatus("Could not remove #" + id);
    }
  });
  return (
    <main class="app">
      <h1 class="title">Todos</h1>
      <div class="composer">
        <input id="new" class="field" value={draft()} placeholder="What needs doing?" onInput={(e: { target: { value: string } }) => setDraft(e.target.value)} onChange={() => add()} />
        <button id="add" class="primary" disabled={draft() === ""} onClick={() => add()}>{"" + (isPending(() => todos()) ? "Saving…" : "Add")}</button>
      </div>
      <Errored fallback={(err) => <p class="error">Could not load your list <button class="ghost" onClick={() => refresh(todos)}>Retry</button></p>}>
      <Loading fallback={<p class="muted">Loading your list…</p>}>
        <div class="tabs">
          <button id="tab-all" class={view() === "all" ? "tab on" : "tab"} onClick={() => setView("all")}>All</button>
          <button id="tab-active" class={view() === "active" ? "tab on" : "tab"} onClick={() => setView("active")}>Active</button>
          <button id="tab-done" class={view() === "done" ? "tab on" : "tab"} onClick={() => setView("done")}>Done</button>
        </div>
        <Show when={visible().length > 0} fallback={<p class="muted">Nothing here</p>}>
          <ul class="list">
            <For each={visible()}>{(t) => (
              <li id={"todo-" + t.id} class={t.done ? "row done" : "row"}>
                <button id={"check-" + t.id} class={t.done ? "check checked" : "check"} onClick={() => toggle(t.id, !t.done)}>{"" + (t.done ? "✓" : "")}</button>
                <span class="label">{t.title}</span>
                <button id={"remove-" + t.id} class="ghost" onClick={() => remove(t.id)}>Remove</button>
              </li>
            )}</For>
          </ul>
        </Show>
        <p class="footer">{left()} left</p>
      </Loading>
      </Errored>
      <p class="status">{status()}</p>
    </main>
  );
}
