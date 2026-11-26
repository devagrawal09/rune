//! Weather: city tabs over a slow forecast service, a unit switch that only changes text, a
//! refresh that dims what is shown until the new forecast lands, and a city whose first load
//! fails. `cargo run -p solid-desktop --bin weather`.

#![forbid(unsafe_code)]

use serde_json::{Value as J, json};
use solid_desktop::{Backend, Reply, Solid};

/// City, sky now, temperature now (°C), and five days of (sky, low, high).
type City = (&'static str, &'static str, i64, [(&'static str, i64, i64); 5]);

const CITIES: [City; 5] = [
    ("Lisbon", "Sunny with a light sea breeze", 21, [("Sunny", 15, 23), ("Sunny", 16, 25), ("Clouds", 15, 22), ("Showers", 14, 19), ("Sunny", 15, 24)]),
    ("Oslo", "Low cloud, dry", 6, [("Clouds", 2, 7), ("Rain", 3, 8), ("Rain", 4, 9), ("Clouds", 1, 6), ("Snow", -2, 3)]),
    ("Tokyo", "Clear and crisp", 17, [("Clear", 11, 18), ("Clear", 12, 19), ("Clouds", 13, 20), ("Rain", 14, 17), ("Clear", 10, 18)]),
    ("Reykjavik", "Wind from the north", 2, [("Wind", -1, 3), ("Snow", -3, 1), ("Clouds", -2, 2), ("Wind", -1, 4), ("Clear", -4, 0)]),
    ("Austin", "Hot and humid", 31, [("Sunny", 22, 33), ("Storms", 21, 29), ("Sunny", 23, 34), ("Sunny", 24, 35), ("Clouds", 22, 31)]),
];

const DAYS: [&str; 5] = ["Fri", "Sat", "Sun", "Mon", "Tue"];

fn f(c: i64) -> i64 {
    (c * 9 + 160 + 2) / 5
}

/// A forecast service that takes its time, answers a little differently each refresh, and is
/// down for Reykjavik the first time it is asked.
#[derive(Default)]
struct Weather {
    loads: u32,
    reykjavik_failed: bool,
}

impl Backend for Weather {
    fn call(&mut self, host: &str, args: &[J]) -> Reply {
        if host != "loadForecast" {
            return Reply::fail(0.0, format!("no host function {host}"));
        }
        let name = args.first().and_then(J::as_str).unwrap_or_default();
        let Some(&(city, summary, now, days)) = CITIES.iter().find(|c| c.0 == name) else {
            return Reply::fail(0.3, format!("no forecast for {name}"));
        };
        if city == "Reykjavik" && !self.reykjavik_failed {
            self.reykjavik_failed = true;
            return Reply::fail(1.1, "503 from the forecast service");
        }
        self.loads += 1;
        let drift = (self.loads % 3) as i64 - 1;
        let now = now + drift;
        let days: Vec<J> = DAYS.iter().zip(days).map(|(day, (sky, low, high))| {
            json!({"name": day, "sky": sky, "low": low, "lowF": f(low), "high": high, "highF": f(high)})
        }).collect();
        let updated = format!("9:{:02}", 40 + self.loads);
        Reply::ok(0.8 + 0.1 * (self.loads % 3) as f64, json!({
            "city": city, "now": now, "nowF": f(now), "summary": summary, "updated": updated, "days": days
        }))
    }
}

fn init() -> Solid {
    Solid::new(include_str!("../../plans/weather.plan.json"), Box::new(Weather::default()))
}

fn main() {
    rune::run(solid_desktop::app("Weather", (560.0, 540.0), init));
}
