use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::model::{Mode, TestResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputKey {
    Char(char),
    Backspace,
}

#[derive(Clone, Debug)]
pub struct SubmittedWord {
    pub typed: String,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Metrics {
    pub elapsed: Duration,
    pub wpm: f64,
    pub raw_wpm: f64,
    pub accuracy: f64,
    pub correct_chars: u64,
    pub incorrect_chars: u64,
    pub typed_chars: u64,
}

pub struct Engine {
    mode: Mode,
    words: Vec<String>,
    submitted: Vec<SubmittedWord>,
    current_input: String,
    current_word: usize,
    started_at: Option<Instant>,
    finished_at: Option<Instant>,
    printable_keys: u64,
    correct_keypresses: u64,
    committed_correct: u64,
    committed_incorrect: u64,
}

impl Engine {
    pub fn new(mode: Mode, words: Vec<String>) -> Self {
        assert!(!words.is_empty(), "a typing test requires words");
        Self {
            mode,
            words,
            submitted: Vec::new(),
            current_input: String::new(),
            current_word: 0,
            started_at: None,
            finished_at: None,
            printable_keys: 0,
            correct_keypresses: 0,
            committed_correct: 0,
            committed_incorrect: 0,
        }
    }

    pub fn words(&self) -> &[String] {
        &self.words
    }

    pub fn submitted(&self) -> &[SubmittedWord] {
        &self.submitted
    }

    pub fn current_input(&self) -> &str {
        &self.current_input
    }

    pub fn current_word_index(&self) -> usize {
        self.current_word
    }

    pub fn is_started(&self) -> bool {
        self.started_at.is_some()
    }

    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }

    pub fn handle_key_at(&mut self, key: InputKey, now: Instant) {
        if self.is_finished() {
            return;
        }

        match key {
            InputKey::Backspace => {
                self.current_input.pop();
            }
            InputKey::Char(' ') => {
                if !self.current_input.is_empty() {
                    self.ensure_started(now);
                    let finishes_word_test = matches!(self.mode, Mode::Words(target) if self.current_word + 1 == target as usize);
                    self.commit_current(true);
                    if finishes_word_test {
                        self.finished_at = Some(now);
                    }
                }
            }
            InputKey::Char(character) if !character.is_control() => {
                self.ensure_started(now);
                let expected = self.words[self.current_word]
                    .chars()
                    .nth(self.current_input.chars().count());
                self.printable_keys += 1;
                if expected == Some(character) {
                    self.correct_keypresses += 1;
                }
                self.current_input.push(character);

                if let Mode::Words(target) = self.mode {
                    let is_last = self.current_word + 1 == target as usize;
                    if is_last && self.current_input == self.words[self.current_word] {
                        self.commit_current(false);
                        self.finished_at = Some(now);
                    }
                }
            }
            InputKey::Char(_) => {}
        }
    }

    pub fn tick_at(&mut self, now: Instant) {
        let (Mode::Timed(seconds), Some(started)) = (self.mode, self.started_at) else {
            return;
        };
        if !self.is_finished()
            && now.saturating_duration_since(started) >= Duration::from_secs(seconds.into())
        {
            self.finish_partial_word();
            self.finished_at = Some(started + Duration::from_secs(seconds.into()));
        }
    }

    pub fn remaining_at(&self, now: Instant) -> Option<Duration> {
        let Mode::Timed(seconds) = self.mode else {
            return None;
        };
        let Some(started) = self.started_at else {
            return Some(Duration::from_secs(seconds.into()));
        };
        let elapsed = now.saturating_duration_since(started);
        Some(Duration::from_secs(seconds.into()).saturating_sub(elapsed))
    }

    pub fn metrics_at(&self, now: Instant) -> Metrics {
        let elapsed = self.elapsed_at(now);
        let (current_correct, current_incorrect) = self.current_score();
        let correct_chars = self.committed_correct + current_correct;
        let incorrect_chars = self.committed_incorrect + current_incorrect;
        let minutes = elapsed.as_secs_f64() / 60.0;
        let wpm = if minutes > 0.0 {
            correct_chars as f64 / 5.0 / minutes
        } else {
            0.0
        };
        let raw_wpm = if minutes > 0.0 {
            self.printable_keys as f64 / 5.0 / minutes
        } else {
            0.0
        };
        let accuracy = if self.printable_keys > 0 {
            self.correct_keypresses as f64 / self.printable_keys as f64 * 100.0
        } else {
            100.0
        };

        Metrics {
            elapsed,
            wpm,
            raw_wpm,
            accuracy,
            correct_chars,
            incorrect_chars,
            typed_chars: self.printable_keys,
        }
    }

    pub fn result(&self) -> Option<TestResult> {
        let finished = self.finished_at?;
        let metrics = self.metrics_at(finished);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Some(TestResult {
            timestamp,
            mode: self.mode,
            elapsed_ms: metrics.elapsed.as_millis() as u64,
            wpm: metrics.wpm,
            raw_wpm: metrics.raw_wpm,
            accuracy: metrics.accuracy,
            correct_chars: metrics.correct_chars,
            incorrect_chars: metrics.incorrect_chars,
            typed_chars: metrics.typed_chars,
        })
    }

    fn ensure_started(&mut self, now: Instant) {
        self.started_at.get_or_insert(now);
    }

    fn commit_current(&mut self, include_space: bool) {
        let (correct, incorrect) =
            score_word(&self.words[self.current_word], &self.current_input, true);
        self.committed_correct += correct;
        self.committed_incorrect += incorrect;
        if include_space {
            self.printable_keys += 1;
            if self.current_input == self.words[self.current_word] {
                self.correct_keypresses += 1;
                self.committed_correct += 1;
            } else {
                self.committed_incorrect += 1;
            }
        }
        self.submitted.push(SubmittedWord {
            typed: std::mem::take(&mut self.current_input),
        });
        self.current_word += 1;

        if self.current_word >= self.words.len() {
            self.current_word = self.words.len() - 1;
        }
    }

    fn finish_partial_word(&mut self) {
        if self.current_input.is_empty() {
            return;
        }
        let (correct, incorrect) =
            score_word(&self.words[self.current_word], &self.current_input, false);
        self.committed_correct += correct;
        self.committed_incorrect += incorrect;
        self.submitted.push(SubmittedWord {
            typed: std::mem::take(&mut self.current_input),
        });
    }

    fn elapsed_at(&self, now: Instant) -> Duration {
        let Some(started) = self.started_at else {
            return Duration::ZERO;
        };
        self.finished_at
            .unwrap_or(now)
            .saturating_duration_since(started)
    }

    fn current_score(&self) -> (u64, u64) {
        score_word(&self.words[self.current_word], &self.current_input, false)
    }
}

fn score_word(expected: &str, typed: &str, count_missing: bool) -> (u64, u64) {
    let expected: Vec<char> = expected.chars().collect();
    let typed: Vec<char> = typed.chars().collect();
    let mut correct = 0;
    let mut incorrect = 0;

    for (index, character) in typed.iter().enumerate() {
        if expected.get(index) == Some(character) {
            correct += 1;
        } else {
            incorrect += 1;
        }
    }
    if count_missing {
        incorrect += expected.len().saturating_sub(typed.len()) as u64;
    }

    (correct, incorrect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine(mode: Mode) -> Engine {
        Engine::new(mode, vec!["cat".into(), "dog".into(), "sun".into()])
    }

    fn type_text(engine: &mut Engine, text: &str, start: Instant) {
        for character in text.chars() {
            engine.handle_key_at(InputKey::Char(character), start);
        }
    }

    #[test]
    fn starts_on_first_printable_character_not_backspace() {
        let now = Instant::now();
        let mut test = engine(Mode::Timed(30));
        test.handle_key_at(InputKey::Backspace, now);
        assert!(!test.is_started());
        test.handle_key_at(InputKey::Char('c'), now);
        assert!(test.is_started());
    }

    #[test]
    fn backspace_edits_but_preserves_accuracy_penalty() {
        let start = Instant::now();
        let mut test = engine(Mode::Timed(30));
        type_text(&mut test, "cx", start);
        test.handle_key_at(InputKey::Backspace, start);
        test.handle_key_at(InputKey::Char('a'), start);
        let metrics = test.metrics_at(start + Duration::from_secs(1));
        assert_eq!(test.current_input(), "ca");
        assert_eq!(metrics.typed_chars, 3);
        assert!((metrics.accuracy - 66.666).abs() < 0.01);
    }

    #[test]
    fn space_submits_and_counts_missing_characters() {
        let start = Instant::now();
        let mut test = engine(Mode::Timed(30));
        type_text(&mut test, "ca ", start);
        assert_eq!(test.current_word_index(), 1);
        let metrics = test.metrics_at(start + Duration::from_secs(1));
        assert_eq!(metrics.correct_chars, 2);
        assert_eq!(metrics.incorrect_chars, 2);
    }

    #[test]
    fn word_test_finishes_when_last_word_matches() {
        let start = Instant::now();
        let mut test = Engine::new(Mode::Words(2), vec!["cat".into(), "dog".into()]);
        type_text(&mut test, "cat dog", start);
        assert!(test.is_finished());
        assert_eq!(test.result().unwrap().correct_chars, 7);
    }

    #[test]
    fn word_test_can_finish_with_an_incorrect_last_word() {
        let start = Instant::now();
        let mut test = Engine::new(Mode::Words(2), vec!["cat".into(), "dog".into()]);
        type_text(&mut test, "cat dig ", start);
        assert!(test.is_finished());
        let result = test.result().unwrap();
        assert!(result.incorrect_chars > 0);
        assert!(result.accuracy < 100.0);
    }

    #[test]
    fn timed_test_finishes_at_exact_deadline() {
        let start = Instant::now();
        let mut test = engine(Mode::Timed(15));
        type_text(&mut test, "cat", start);
        test.tick_at(start + Duration::from_secs(20));
        let result = test.result().unwrap();
        assert_eq!(result.elapsed_ms, 15_000);
        assert!((result.wpm - 2.4).abs() < f64::EPSILON);
    }
}
