mod app;
mod cli;
mod engine;
mod model;
mod storage;
mod ui;
mod words;

use std::io::{self, IsTerminal};

use clap::Parser;

use crate::{
    app::{App, View},
    cli::Cli,
    storage::Storage,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("wpm: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let override_mode = cli.mode()?;

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err("an interactive terminal is required".into());
    }

    let storage = Storage::discover()?;
    let config = storage.load_config()?;
    let stats = storage.load_stats()?;
    let active_mode = override_mode.unwrap_or(config.mode);
    let initial_view = if cli.settings {
        View::Settings
    } else if cli.stats {
        View::Stats
    } else {
        View::Test
    };

    let app = App::new(config, stats, active_mode, initial_view);
    let app = ui::run(app)?;

    if app.config_dirty {
        storage.save_config(&app.config)?;
    }
    if app.stats_dirty {
        storage.save_stats(&app.stats)?;
    }

    Ok(())
}
