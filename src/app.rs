use std::time::Instant;

use crate::{
    engine::Engine,
    model::{Config, Mode, Stats, TestResult},
    words,
};

pub const TIME_PRESETS: [u16; 4] = [15, 30, 60, 120];
pub const WORD_PRESETS: [u16; 4] = [10, 25, 50, 100];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum View {
    Test,
    Results,
    Settings,
    Stats,
}

pub struct App {
    pub config: Config,
    pub stats: Stats,
    pub active_mode: Mode,
    pub view: View,
    pub engine: Engine,
    pub last_result: Option<TestResult>,
    pub settings_mode: Mode,
    pub settings_row: usize,
    pub config_dirty: bool,
    pub stats_dirty: bool,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config, stats: Stats, active_mode: Mode, initial_view: View) -> Self {
        let settings_mode = config.mode;
        Self {
            config,
            stats,
            active_mode,
            view: initial_view,
            engine: new_engine(active_mode),
            last_result: None,
            settings_mode,
            settings_row: 0,
            config_dirty: false,
            stats_dirty: false,
            should_quit: false,
        }
    }

    pub fn restart(&mut self) {
        self.engine = new_engine(self.active_mode);
        self.last_result = None;
        self.view = View::Test;
    }

    pub fn complete_if_finished(&mut self) {
        if self.view != View::Test || !self.engine.is_finished() {
            return;
        }
        if let Some(result) = self.engine.result() {
            self.stats.record(result.clone());
            self.last_result = Some(result);
            self.stats_dirty = true;
            self.view = View::Results;
        }
    }

    pub fn open_settings(&mut self) {
        self.settings_mode = self.config.mode;
        self.settings_row = 0;
        self.view = View::Settings;
    }

    pub fn adjust_setting(&mut self, direction: isize) {
        if self.settings_row == 0 {
            self.settings_mode = match self.settings_mode {
                Mode::Timed(_) => Mode::Words(25),
                Mode::Words(_) => Mode::Timed(30),
            };
            return;
        }

        let presets = match self.settings_mode {
            Mode::Timed(_) => &TIME_PRESETS,
            Mode::Words(_) => &WORD_PRESETS,
        };
        let current = match self.settings_mode {
            Mode::Timed(value) | Mode::Words(value) => value,
        };
        let index = presets
            .iter()
            .position(|value| *value == current)
            .unwrap_or(0);
        let next = (index as isize + direction).rem_euclid(presets.len() as isize) as usize;
        self.settings_mode = match self.settings_mode {
            Mode::Timed(_) => Mode::Timed(presets[next]),
            Mode::Words(_) => Mode::Words(presets[next]),
        };
    }

    pub fn save_settings(&mut self) {
        self.config.mode = self.settings_mode;
        self.active_mode = self.settings_mode;
        self.config_dirty = true;
        self.restart();
    }

    pub fn leave_auxiliary_view(&mut self) {
        self.view = if self.last_result.is_some() {
            View::Results
        } else {
            View::Test
        };
    }

    pub fn tick(&mut self, now: Instant) {
        if self.view == View::Test {
            self.engine.tick_at(now);
            self.complete_if_finished();
        }
    }
}

fn new_engine(mode: Mode) -> Engine {
    let count = match mode {
        Mode::Timed(_) => 1_000,
        Mode::Words(count) => count as usize,
    };
    Engine::new(mode, words::generate(count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_override_does_not_replace_saved_mode() {
        let config = Config {
            mode: Mode::Timed(30),
            ..Config::default()
        };
        let app = App::new(config, Stats::default(), Mode::Words(50), View::Test);
        assert_eq!(app.active_mode, Mode::Words(50));
        assert_eq!(app.config.mode, Mode::Timed(30));
        assert!(!app.config_dirty);
    }

    #[test]
    fn settings_are_applied_and_persisted() {
        let mut app = App::new(
            Config::default(),
            Stats::default(),
            Mode::Timed(30),
            View::Settings,
        );
        app.settings_mode = Mode::Words(25);
        app.save_settings();
        assert_eq!(app.config.mode, Mode::Words(25));
        assert_eq!(app.active_mode, Mode::Words(25));
        assert!(app.config_dirty);
    }
}
