use clap::{ArgGroup, Parser};

use crate::model::Mode;

const TIME_PRESETS: &[u16] = &[15, 30, 60, 120];
const WORD_PRESETS: &[u16] = &[10, 25, 50, 100];

#[derive(Debug, Parser)]
#[command(
    name = "wpm",
    version,
    about = "A fast, focused typing test for your terminal",
    group(ArgGroup::new("screen").args(["settings", "stats"]).multiple(false))
)]
pub struct Cli {
    /// Run a timed test (15, 30, 60, or 120 seconds)
    #[arg(long, value_name = "SECONDS", conflicts_with_all = ["words", "settings", "stats"])]
    pub time: Option<u16>,

    /// Run a word-count test (10, 25, 50, or 100 words)
    #[arg(long, value_name = "COUNT", conflicts_with_all = ["time", "settings", "stats"])]
    pub words: Option<u16>,

    /// Open the settings screen
    #[arg(long)]
    pub settings: bool,

    /// Open typing history and personal bests
    #[arg(long)]
    pub stats: bool,
}

impl Cli {
    pub fn mode(&self) -> Result<Option<Mode>, String> {
        if let Some(seconds) = self.time {
            if !TIME_PRESETS.contains(&seconds) {
                return Err(format!(
                    "unsupported time {seconds}; choose 15, 30, 60, or 120 seconds"
                ));
            }
            return Ok(Some(Mode::Timed(seconds)));
        }

        if let Some(words) = self.words {
            if !WORD_PRESETS.contains(&words) {
                return Err(format!(
                    "unsupported word count {words}; choose 10, 25, 50, or 100"
                ));
            }
            return Ok(Some(Mode::Words(words)));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_presets() {
        let cli = Cli::try_parse_from(["wpm", "--time", "60"]).unwrap();
        assert_eq!(cli.mode().unwrap(), Some(Mode::Timed(60)));

        let cli = Cli::try_parse_from(["wpm", "--words", "13"]).unwrap();
        assert!(cli.mode().unwrap_err().contains("unsupported word count"));
    }

    #[test]
    fn conflicting_modes_are_rejected() {
        assert!(Cli::try_parse_from(["wpm", "--time", "30", "--words", "25"]).is_err());
    }
}
