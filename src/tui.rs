// SPDX-License-Identifier: GPL-3.0-only
use std::{io::Stdout, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::{
    app::App,
    theme::{self, logo_plain},
};

///Runs the ratatui event loop until the cat naps.
pub fn run_tui(app: &mut App) -> color_eyre::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // pounce once on open so the panes are not empty
    app.run_once();
    loop {
        terminal.draw(|f| draw(f, app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            handle_key(app, key.code, key.modifiers);
        }
        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

// routes keys, with a tiny vim-style input for new sunbeams
fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    if app.input_mode {
        match code {
            KeyCode::Esc => app.cancel_input(),
            KeyCode::Enter => app.submit_input(),
            KeyCode::Backspace => {
                app.input.pop();
            }
            KeyCode::Char(c) => {
                app.input.push(c);
            }
            _ => {}
        }
        return;
    }
    if mods.contains(KeyModifiers::CONTROL)
        && let KeyCode::Char('c') = code
    {
        app.should_quit = true;
        return;
    }
    match code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('r') => app.run_once(),
        KeyCode::Char('i') => app.toggle_ir(),
        KeyCode::Char('t') => app.dump_tokens(),
        KeyCode::Char('o') => app.enter_input(),
        KeyCode::Up | KeyCode::Char('k') => app.scroll_by(-3),
        KeyCode::Down | KeyCode::Char('j') => app.scroll_by(3),
        KeyCode::PageUp => app.scroll_by(-15),
        KeyCode::PageDown => app.scroll_by(15),
        _ => {}
    }
}

///Draws header, sunbeam + yarn ball panes, and footer.
fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    // header keeps the only pixelblock accent
    let file = app
        .file
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "no sunbeam yet…".to_string());
    let header = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            format!("{} — slynx pounce station", logo_plain()),
            theme::title_style(),
        )]),
        Line::from(vec![
            Span::styled(app.status_text(), theme::dim_style()),
            Span::raw("  "),
            Span::styled(file, theme::border_style()),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(theme::border_style())
            .title(" lyx-meow "),
    );
    f.render_widget(header, rows[0]);

    // body without pixelblocks, clean green borders
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(rows[1]);
    let source = Paragraph::new(app.source.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border_style())
                .title(" sunbeam (source) "),
        )
        .wrap(Wrap { trim: false })
        .scroll((0, 0));
    f.render_widget(source, cols[0]);
    let output = Paragraph::new(app.output_text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border_style())
                .title(" yarn ball (output) "),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    f.render_widget(output, cols[1]);

    // footer with keys and one short yarn strip
    let footer_text = if app.input_mode {
        format!("new sunbeam: {}▊ (enter to pounce, esc to hide)", app.input)
    } else {
        format!(
            "r pounce · i knead IR · t sniff tokens · o new sunbeam · ↑↓ scroll · q nap  {}",
            crate::theme::ACCENT
        )
    };
    let footer = Paragraph::new(footer_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(theme::border_style())
                .title(" paws "),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(footer, rows[2]);
}

#[allow(dead_code)]
// keeps stdout type visible for readers
fn _backend_note(_t: &Terminal<CrosstermBackend<Stdout>>) {}

#[cfg(test)]
mod tests {
    use super::super::app::App;

    #[test]
    fn output_prefers_cat_words_when_ir_hidden() {
        let mut app = App::new(None, None);
        app.show_ir = false;
        app.report = Some(crate::runner::RunReport {
            ok: true,
            ms: 1,
            ir_text: "fn".to_string(),
            error: String::new(),
            file: "a.syx".into(),
        });
        assert!(app.output_text().contains("curled up"));
    }
}
