use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Mode {
    Timed(u16),
    Words(u16),
}

impl Mode {
    pub fn label(self) -> String {
        match self {
            Self::Timed(seconds) => format!("{seconds} seconds"),
            Self::Words(words) => format!("{words} words"),
        }
    }

    pub fn key(self) -> String {
        match self {
            Self::Timed(seconds) => format!("time:{seconds}"),
            Self::Words(words) => format!("words:{words}"),
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::Timed(30)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub mode: Mode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            mode: Mode::default(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TestResult {
    pub timestamp: u64,
    pub mode: Mode,
    pub elapsed_ms: u64,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub correct_chars: u64,
    pub incorrect_chars: u64,
    pub typed_chars: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PersonalBest {
    pub wpm: f64,
    pub accuracy: f64,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Stats {
    pub schema_version: u32,
    pub total_tests: u64,
    pub total_time_ms: u64,
    pub total_typed_chars: u64,
    pub personal_bests: BTreeMap<String, PersonalBest>,
    pub recent_results: Vec<TestResult>,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            total_tests: 0,
            total_time_ms: 0,
            total_typed_chars: 0,
            personal_bests: BTreeMap::new(),
            recent_results: Vec::new(),
        }
    }
}

impl Stats {
    pub fn record(&mut self, result: TestResult) {
        self.total_tests += 1;
        self.total_time_ms += result.elapsed_ms;
        self.total_typed_chars += result.typed_chars;

        let key = result.mode.key();
        let is_best = self
            .personal_bests
            .get(&key)
            .map(|best| {
                result.wpm > best.wpm
                    || ((result.wpm - best.wpm).abs() < f64::EPSILON
                        && result.accuracy > best.accuracy)
            })
            .unwrap_or(true);
        if is_best {
            self.personal_bests.insert(
                key,
                PersonalBest {
                    wpm: result.wpm,
                    accuracy: result.accuracy,
                    timestamp: result.timestamp,
                },
            );
        }

        self.recent_results.insert(0, result);
        self.recent_results.truncate(1_000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(wpm: f64, accuracy: f64) -> TestResult {
        TestResult {
            timestamp: 1,
            mode: Mode::Timed(30),
            elapsed_ms: 30_000,
            wpm,
            raw_wpm: wpm,
            accuracy,
            correct_chars: 100,
            incorrect_chars: 0,
            typed_chars: 100,
        }
    }

    #[test]
    fn records_totals_history_and_personal_best() {
        let mut stats = Stats::default();
        stats.record(result(50.0, 99.0));
        stats.record(result(40.0, 100.0));
        stats.record(result(50.0, 100.0));

        assert_eq!(stats.total_tests, 3);
        assert_eq!(stats.recent_results.len(), 3);
        let best = &stats.personal_bests["time:30"];
        assert_eq!(best.wpm, 50.0);
        assert_eq!(best.accuracy, 100.0);
    }
}
