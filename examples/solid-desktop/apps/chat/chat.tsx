import { action, createMemo, createOptimistic, createSignal, Errored, For, Loading, refresh, Show } from "solid-js";
import { awaitReply, loadMessages, sendMessage } from "./api";

export default function Chat() {
  const messages = createMemo(() => loadMessages());
  const [draft, setDraft] = createSignal("");
  const [sending, setSending] = createOptimistic("");
  const [typing, setTyping] = createOptimistic(false);
  const [failed, setFailed] = createSignal("");
  const send = action(function* (text: string) {
    setSending(text);
    setFailed("");
    try {
      const id: number = yield sendMessage(text);
      refresh(messages);
      setTyping(true);
      const ok: boolean = yield awaitReply(id);
      refresh(messages);
    } catch (error) {
      setFailed(text);
    }
  });
  return (
    <main class="app">
      <div class="bar">
        <h1 class="title">Ada</h1>
        <span class="spacer"></span>
        <small>{"" + (typing() ? "typing…" : "online")}</small>
      </div>
      <Errored fallback={(err) => <p class="error">Could not load the conversation <button class="ghost" onClick={() => refresh(messages)}>Retry</button></p>}>
        <Loading fallback={<p class="muted">Loading the conversation…</p>}>
          <div class="thread">
            <For each={messages()}>{(m) => (
              <div id={"line-" + m.id} class={m.mine ? "mine" : "line"}>
                <div class={m.mine ? "bubble me" : "bubble them"}>
                  <span>{m.text}</span>
                </div>
              </div>
            )}</For>
            <Show when={sending() !== ""}>
              <div id="sending" class="mine">
                <div class={typing() ? "bubble me" : "bubble me pending"}><span>{sending()}</span></div>
              </div>
            </Show>
            <Show when={typing()}>
              <div id="typing" class="line"><div class="bubble them typing"><span>Ada is typing…</span></div></div>
            </Show>
            <Show when={failed() !== ""}>
              <div id="failed" class="mine">
                <p class="error">{"Not sent: " + failed()} <button id="retry" class="ghost" onClick={() => setDraft(failed())}>Edit</button></p>
              </div>
            </Show>
          </div>
        </Loading>
      </Errored>
      <div class="composer">
        <input id="message" class="field" value={sending() !== "" ? "" : draft()} placeholder="Message Ada" onInput={(e: { target: { value: string } }) => setDraft(e.target.value)} onChange={() => { send(draft()); setDraft(""); }} />
        <button id="send" class="primary" disabled={draft() === "" || sending() !== ""} onClick={() => { send(draft()); setDraft(""); }}>Send</button>
      </div>
    </main>
  );
}
