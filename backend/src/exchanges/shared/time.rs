// src/common/timespec.rs
use chrono::Utc;

#[derive(Clone, Debug)]
pub enum TimeSpec {
    /// Fixed time window between two millisecond timestamps
    Between { start_ms: u64, end_ms: u64 },

    /// Look back `h` hours from now until now
    LookbackHours(u64),

    /// Use last timestamp if available, otherwise look back `h` hours
    SinceLastOrLookbackHours(u64),

    /// Interval-based specifier (e.g. "1m", "1h", "1d")
    Interval(String),
}

impl TimeSpec {
    /// Resolve into a concrete range (`start_ms`, `end_ms`).
    /// Optionally use last known timestamp in milliseconds.
    pub fn resolve(&self, last_ts_ms: Option<i64>) -> (u64, u64) {
        let now_ms = Utc::now().timestamp_millis() as u64;
        match self {
            TimeSpec::Between { start_ms, end_ms } => (*start_ms, *end_ms),

            TimeSpec::LookbackHours(h) => {
                (now_ms.saturating_sub(h.saturating_mul(3_600_000)), now_ms)
            }

            TimeSpec::SinceLastOrLookbackHours(h) => {
                if let Some(last_ms) = last_ts_ms {
                    ((last_ms as u64).saturating_add(1), now_ms)
                } else {
                    (now_ms.saturating_sub(h.saturating_mul(3_600_000)), now_ms)
                }
            }

            // For interval mode, you typically pass this to the API layer
            // but if you need a resolve() output, provide a default window.
            TimeSpec::Interval(interval) => match interval.as_str() {
                "1m" => (now_ms.saturating_sub(60_000), now_ms),
                "1h" => (now_ms.saturating_sub(3_600_000), now_ms),
                "1d" => (now_ms.saturating_sub(86_400_000), now_ms),
                _ => (now_ms.saturating_sub(3_600_000), now_ms),
            },
        }
    }
}
