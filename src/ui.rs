use std::{
    env,
    io::{self, stdout, Stdout},
    time::{Duration, Instant},
};

use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame, Terminal,
};

use crate::{
    app::{App, View},
    engine::{Engine, InputKey},
    model::{Mode, TestResult},
};

type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn run(mut app: App) -> io::Result<App> {
    let mut terminal = TerminalGuard::enter()?;
    let no_color = env::var_os("NO_COLOR").is_some();

    while !app.should_quit {
        let now = Instant::now();
        app.tick(now);
        terminal
            .terminal
            .draw(|frame| draw(frame, &app, now, no_color))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key)
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
                {
                    handle_key(&mut app, key, Instant::now());
                    app.complete_if_finished();
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }

    Ok(app)
}

struct TerminalGuard {
    terminal: Tui,
}

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut output = stdout();
        if let Err(error) = execute!(output, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        let backend = CrosstermBackend::new(output);
        let terminal = match Terminal::new(backend) {
            Ok(terminal) => terminal,
            Err(error) => {
                let _ = disable_raw_mode();
                let _ = execute!(stdout(), Show, LeaveAlternateScreen);
                return Err(error);
            }
        };
        let mut guard = Self { terminal };
        guard.terminal.clear()?;
        Ok(guard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), Show, LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn handle_key(app: &mut App, key: KeyEvent, now: Instant) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.view {
        View::Test => match key.code {
            KeyCode::Esc => app.should_quit = true,
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => app.restart(),
            KeyCode::Tab if !app.engine.is_started() => app.open_settings(),
            KeyCode::Backspace => app.engine.handle_key_at(InputKey::Backspace, now),
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                app.engine.handle_key_at(InputKey::Char(character), now)
            }
            _ => {}
        },
        View::Results => match key.code {
            KeyCode::Enter => app.restart(),
            KeyCode::Tab => app.open_settings(),
            KeyCode::Char('h') => app.view = View::Stats,
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            _ => {}
        },
        View::Settings => match key.code {
            KeyCode::Up => app.settings_row = app.settings_row.saturating_sub(1),
            KeyCode::Down => app.settings_row = (app.settings_row + 1).min(1),
            KeyCode::Left => app.adjust_setting(-1),
            KeyCode::Right => app.adjust_setting(1),
            KeyCode::Enter => app.save_settings(),
            KeyCode::Esc => app.leave_auxiliary_view(),
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        },
        View::Stats => match key.code {
            KeyCode::Esc => app.leave_auxiliary_view(),
            KeyCode::Tab => app.open_settings(),
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        },
    }
}

fn draw(frame: &mut Frame, app: &App, now: Instant, no_color: bool) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    if area.width < 46 || area.height < 12 {
        frame.render_widget(
            Paragraph::new("TypeTTY needs a terminal at least 46 × 12")
                .alignment(Alignment::Center),
            centered(area, 3, area.width.saturating_sub(2)),
        );
        return;
    }

    let theme = Theme::new(no_color);
    let shell = centered(area, area.height.min(28), area.width.min(104));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(7),
            Constraint::Length(2),
        ])
        .split(shell);

    let title = Line::from(vec![
        Span::styled("Type", theme.title()),
        Span::styled("TTY", theme.accent()),
        Span::styled("  ·  terminal typing", theme.muted()),
    ]);
    frame.render_widget(
        Paragraph::new(title).alignment(Alignment::Center),
        chunks[0],
    );

    match app.view {
        View::Test => draw_test(frame, chunks[1], app, now, &theme),
        View::Results => draw_results(frame, chunks[1], app.last_result.as_ref(), &theme),
        View::Settings => draw_settings(frame, chunks[1], app, &theme),
        View::Stats => draw_stats(frame, chunks[1], app, &theme),
    }

    let footer = match app.view {
        View::Test if !app.engine.is_started() => "Tab settings  ·  Esc quit  ·  start typing",
        View::Test => "Ctrl+R restart  ·  Esc quit",
        View::Results => "Enter restart  ·  Tab settings  ·  h history  ·  q quit",
        View::Settings => "↑↓ select  ·  ←→ change  ·  Enter save  ·  Esc back",
        View::Stats => "Tab settings  ·  Esc back  ·  q quit",
    };
    frame.render_widget(
        Paragraph::new(footer)
            .style(theme.muted())
            .alignment(Alignment::Center),
        chunks[2],
    );
}

fn draw_test(frame: &mut Frame, area: Rect, app: &App, now: Instant, theme: &Theme) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(area);
    let metrics = app.engine.metrics_at(now);

    let status = match app.active_mode {
        Mode::Timed(_) => {
            let remaining = app.engine.remaining_at(now).unwrap_or_default();
            let seconds = remaining.as_secs_f64().ceil() as u64;
            format!("{seconds}s")
        }
        Mode::Words(target) => format!(
            "{}/{} words",
            app.engine.current_word_index().min(target as usize) + 1,
            target
        ),
    };
    let live = if app.engine.is_started() {
        format!(
            "{}    {:>3.0} wpm    {:>3.0}% acc",
            status, metrics.wpm, metrics.accuracy
        )
    } else {
        format!("{}    ready", app.active_mode.label())
    };
    frame.render_widget(
        Paragraph::new(live)
            .style(theme.accent())
            .alignment(Alignment::Center),
        chunks[0],
    );

    let word_area = Rect {
        x: chunks[1].x + 2,
        y: chunks[1].y,
        width: chunks[1].width.saturating_sub(4),
        height: chunks[1].height,
    };
    let lines = visible_word_lines(&app.engine, word_area.width, word_area.height, theme);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .alignment(Alignment::Left),
        word_area,
    );

    if !app.engine.is_started() {
        frame.render_widget(
            Paragraph::new("timer starts with your first key")
                .style(theme.muted())
                .alignment(Alignment::Center),
            chunks[2],
        );
    }
}

fn draw_results(frame: &mut Frame, area: Rect, result: Option<&TestResult>, theme: &Theme) {
    let Some(result) = result else {
        frame.render_widget(
            Paragraph::new("No result yet").alignment(Alignment::Center),
            area,
        );
        return;
    };
    let box_area = centered(area, 12.min(area.height), 62.min(area.width));
    let lines = vec![
        Line::from(Span::styled("test complete", theme.title())),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{:>3.0}", result.wpm), theme.score()),
            Span::styled(" wpm", theme.muted()),
            Span::raw("       "),
            Span::styled(format!("{:>3.0}%", result.accuracy), theme.score()),
            Span::styled(" accuracy", theme.muted()),
        ]),
        Line::from(""),
        result_line("raw", format!("{:.0} wpm", result.raw_wpm), theme),
        result_line(
            "characters",
            format!("{} / {}", result.correct_chars, result.incorrect_chars),
            theme,
        ),
        result_line(
            "elapsed",
            format!("{:.1}s", result.elapsed_ms as f64 / 1_000.0),
            theme,
        ),
        result_line("mode", result.mode.label(), theme),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.muted()),
            )
            .alignment(Alignment::Center),
        box_area,
    );
}

fn result_line(label: &str, value: String, theme: &Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), theme.muted()),
        Span::styled(value, theme.normal()),
    ])
}

fn draw_settings(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let box_area = centered(area, 11.min(area.height), 54.min(area.width));
    let (kind, value) = match app.settings_mode {
        Mode::Timed(seconds) => ("timed", format!("{seconds} seconds")),
        Mode::Words(words) => ("words", format!("{words} words")),
    };
    let lines = vec![
        Line::from(Span::styled("settings", theme.title())),
        Line::from(""),
        setting_line("mode", kind, app.settings_row == 0, theme),
        Line::from(""),
        setting_line("length", &value, app.settings_row == 1, theme),
        Line::from(""),
        Line::from(Span::styled(
            "changes become your new default",
            theme.muted(),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.muted()),
            )
            .alignment(Alignment::Center),
        box_area,
    );
}

fn setting_line(label: &str, value: &str, selected: bool, theme: &Theme) -> Line<'static> {
    let marker = if selected { "› " } else { "  " };
    let value_style = if selected {
        theme.selected()
    } else {
        theme.normal()
    };
    Line::from(vec![
        Span::styled(format!("{marker}{label:<8}"), theme.muted()),
        Span::styled(format!(" {value} "), value_style),
    ])
}

fn draw_stats(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mut lines = vec![
        Line::from(Span::styled("history", theme.title())),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{}", app.stats.total_tests), theme.score()),
            Span::styled(" tests     ", theme.muted()),
            Span::styled(
                format!("{:.1}", app.stats.total_time_ms as f64 / 60_000.0),
                theme.score(),
            ),
            Span::styled(" minutes     ", theme.muted()),
            Span::styled(format!("{}", app.stats.total_typed_chars), theme.score()),
            Span::styled(" keys", theme.muted()),
        ]),
        Line::from(""),
        Line::from(Span::styled("personal bests", theme.accent())),
    ];

    if app.stats.personal_bests.is_empty() {
        lines.push(Line::from(Span::styled(
            "complete a test to set one",
            theme.muted(),
        )));
    } else {
        for (mode, best) in app.stats.personal_bests.iter().take(4) {
            lines.push(Line::from(vec![
                Span::styled(format!("{mode:<12}"), theme.muted()),
                Span::styled(format!("{:>3.0} wpm", best.wpm), theme.normal()),
                Span::styled(format!("  {:>3.0}%", best.accuracy), theme.muted()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("recent", theme.accent())));
    for result in app.stats.recent_results.iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled(format!("{:<12}", result.mode.key()), theme.muted()),
            Span::styled(format!("{:>3.0} wpm", result.wpm), theme.normal()),
            Span::styled(format!("  {:>3.0}%", result.accuracy), theme.muted()),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn visible_word_lines(
    engine: &Engine,
    width: u16,
    height: u16,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let width = width.max(1) as usize;
    let mut rows: Vec<Vec<usize>> = vec![Vec::new()];
    let mut row_width = 0usize;
    let mut active_row = 0usize;

    for (index, word) in engine.words().iter().enumerate() {
        let word_width = word.chars().count() + 1;
        if row_width > 0 && row_width + word_width > width {
            rows.push(Vec::new());
            row_width = 0;
        }
        if index == engine.current_word_index() {
            active_row = rows.len() - 1;
        }
        rows.last_mut().expect("at least one row").push(index);
        row_width += word_width;
    }

    let visible_count = height.max(1) as usize;
    let start = active_row.saturating_sub(visible_count / 2);
    rows.into_iter()
        .skip(start)
        .take(visible_count)
        .map(|indices| {
            let spans = indices
                .into_iter()
                .flat_map(|index| word_spans(engine, index, theme))
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect()
}

fn word_spans(engine: &Engine, index: usize, theme: &Theme) -> Vec<Span<'static>> {
    let expected = &engine.words()[index];
    let typed = if index < engine.submitted().len() {
        Some(engine.submitted()[index].typed.as_str())
    } else if index == engine.current_word_index() {
        Some(engine.current_input())
    } else {
        None
    };
    let submitted = index < engine.submitted().len();
    let active = index == engine.current_word_index() && !engine.is_finished();
    let typed_chars: Vec<char> = typed.unwrap_or_default().chars().collect();
    let expected_chars: Vec<char> = expected.chars().collect();
    let mut spans = Vec::new();

    for (position, character) in expected_chars.iter().enumerate() {
        let style = match typed_chars.get(position) {
            Some(typed) if typed == character => theme.correct(),
            Some(_) => theme.error(),
            None if submitted => theme.missed(),
            None if active && position == typed_chars.len() => theme.cursor(),
            None => theme.muted(),
        };
        spans.push(Span::styled(character.to_string(), style));
    }
    for character in typed_chars.iter().skip(expected_chars.len()) {
        spans.push(Span::styled(character.to_string(), theme.extra()));
    }

    let space_style = if submitted {
        if typed.unwrap_or_default() == expected {
            theme.correct()
        } else {
            theme.error()
        }
    } else if active && typed_chars.len() >= expected_chars.len() {
        theme.cursor()
    } else {
        theme.muted()
    };
    spans.push(Span::styled(" ", space_style));
    spans
}

fn centered(area: Rect, height: u16, width: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

struct Theme {
    no_color: bool,
}

impl Theme {
    fn new(no_color: bool) -> Self {
        Self { no_color }
    }

    fn color(&self, color: Color) -> Style {
        if self.no_color {
            Style::default()
        } else {
            Style::default().fg(color)
        }
    }

    fn normal(&self) -> Style {
        self.color(Color::White)
    }

    fn title(&self) -> Style {
        self.normal().add_modifier(Modifier::BOLD)
    }

    fn accent(&self) -> Style {
        self.color(Color::Yellow).add_modifier(Modifier::BOLD)
    }

    fn muted(&self) -> Style {
        self.color(Color::DarkGray)
    }

    fn correct(&self) -> Style {
        self.color(Color::Green)
    }

    fn error(&self) -> Style {
        self.color(Color::Red).add_modifier(Modifier::UNDERLINED)
    }

    fn missed(&self) -> Style {
        self.color(Color::Red).add_modifier(Modifier::DIM)
    }

    fn extra(&self) -> Style {
        self.color(Color::Red).add_modifier(Modifier::REVERSED)
    }

    fn cursor(&self) -> Style {
        self.color(Color::Yellow)
            .add_modifier(Modifier::UNDERLINED | Modifier::BOLD)
    }

    fn selected(&self) -> Style {
        self.color(Color::Black)
            .bg(if self.no_color {
                Color::Reset
            } else {
                Color::Yellow
            })
            .add_modifier(Modifier::BOLD | Modifier::REVERSED)
    }

    fn score(&self) -> Style {
        self.color(Color::Yellow).add_modifier(Modifier::BOLD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_without_escaping_parent() {
        let parent = Rect::new(5, 7, 20, 10);
        assert_eq!(centered(parent, 4, 10), Rect::new(10, 10, 10, 4));
        assert_eq!(centered(parent, 40, 50), parent);
    }

    #[test]
    fn current_word_remains_in_visible_rows() {
        let words = (0..50).map(|_| "hello".to_owned()).collect();
        let mut engine = Engine::new(Mode::Timed(30), words);
        let now = Instant::now();
        for _ in 0..20 {
            for character in "hello ".chars() {
                engine.handle_key_at(InputKey::Char(character), now);
            }
        }
        let lines = visible_word_lines(&engine, 20, 3, &Theme::new(true));
        assert_eq!(lines.len(), 3);
    }
}
