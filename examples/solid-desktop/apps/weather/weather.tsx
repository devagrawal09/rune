import { createMemo, createSignal, Errored, For, isPending, Loading, refresh } from "solid-js";
import { loadForecast } from "./api";

export default function Weather() {
  const [city, setCity] = createSignal("Lisbon");
  const [unit, setUnit] = createSignal("C");
  const forecast = createMemo(() => loadForecast(city()));
  return (
    <main class="app">
      <div class="bar">
        <h1 class="title">Weather</h1>
        <span class="spacer"></span>
        <div class="tabs">
          <button id="unit-c" class={unit() === "C" ? "tab on" : "tab"} onClick={() => setUnit("C")}>°C</button>
          <button id="unit-f" class={unit() === "F" ? "tab on" : "tab"} onClick={() => setUnit("F")}>°F</button>
        </div>
      </div>
      <div class="cities">
        <button id="city-Lisbon" class={city() === "Lisbon" ? "tab on" : "tab"} onClick={() => setCity("Lisbon")}>Lisbon</button>
        <button id="city-Oslo" class={city() === "Oslo" ? "tab on" : "tab"} onClick={() => setCity("Oslo")}>Oslo</button>
        <button id="city-Tokyo" class={city() === "Tokyo" ? "tab on" : "tab"} onClick={() => setCity("Tokyo")}>Tokyo</button>
        <button id="city-Reykjavik" class={city() === "Reykjavik" ? "tab on" : "tab"} onClick={() => setCity("Reykjavik")}>Reykjavik</button>
        <button id="city-Austin" class={city() === "Austin" ? "tab on" : "tab"} onClick={() => setCity("Austin")}>Austin</button>
      </div>
      <Errored fallback={(err) => <p class="error">The forecast service did not answer <button id="retry" class="ghost" onClick={() => refresh(forecast)}>Retry</button></p>}>
        <Loading fallback={<p class="muted">Loading the forecast…</p>}>
          <div class={isPending(() => forecast()) ? "hero stale" : "hero"}>
            <div class="bar">
              <h2>{forecast().city}</h2>
              <span class="spacer"></span>
              <small>{"Updated " + forecast().updated}</small>
              <button id="refresh" class="ghost" onClick={() => refresh(forecast)}>Refresh</button>
            </div>
            <p class="big">{"" + (unit() === "C" ? forecast().now : forecast().nowF) + "°"}</p>
            <p class="muted">{forecast().summary}</p>
          </div>
          <div class="days">
            <For each={forecast().days}>{(d) => (
              <div id={"day-" + d.name} class="day">
                <span class="name">{d.name}</span>
                <span class="label">{d.sky}</span>
                <span class="low">{"" + (unit() === "C" ? d.low : d.lowF) + "°"}</span>
                <span class={d.high >= 25 ? "high hot" : "high"}>{"" + (unit() === "C" ? d.high : d.highF) + "°"}</span>
              </div>
            )}</For>
          </div>
        </Loading>
      </Errored>
    </main>
  );
}
