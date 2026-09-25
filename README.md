# TypeTTY

TypeTTY is a fast, focused typing test that lives in your terminal. Run `wpm`,
start typing, and get immediate feedback without opening a browser or creating
an account.

## Install

macOS and Linux users can install the latest release without Rust or elevated
permissions:

```sh
curl -fsSL https://raw.githubusercontent.com/omanar42/TypeTTY/main/install.sh | sh
```

The installer places `wpm` in `~/.local/bin` and, when needed, adds that
directory to `.zshrc` or `.bashrc`. Open a new terminal afterward, or run the
activation command printed by the installer.

To install into another directory:

```sh
curl -fsSL https://raw.githubusercontent.com/omanar42/TypeTTY/main/install.sh \
  | WPM_INSTALL_DIR="$HOME/bin" sh
```

## Use

```text
wpm                 Start with the remembered setup
wpm --time 30       Run a 30-second test once
wpm --words 50      Run a 50-word test once
wpm --settings      Open settings
wpm --stats         Open history and personal bests
```

Timed presets are 15, 30, 60, and 120 seconds. Word-count presets are 10, 25,
50, and 100 words. Command-line overrides apply to one run; changes saved in
the settings screen become the default.

Inside a test:

- Start typing to start the timer.
- Use Backspace to edit the current word and Space to submit it.
- Press `Ctrl+R` to restart or `Esc` to quit.
- Press Tab before typing to open settings.

TypeTTY stores settings in `$XDG_CONFIG_HOME/typetty` (or
`~/.config/typetty`) and statistics in `$XDG_DATA_HOME/typetty` (or
`~/.local/share/typetty`). It has no accounts, telemetry, or network features.

## Build from source

Rust 1.74 or newer is required.

```sh
cargo build --release --locked
./target/release/wpm
```

Run the quality checks with:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
shellcheck install.sh tests/install_test.sh
```

## License

TypeTTY is available under the [MIT License](LICENSE).
