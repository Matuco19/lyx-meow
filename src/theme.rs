// SPDX-License-Identifier: GPL-3.0-only
use ratatui::style::{Color, Modifier, Style};

///Green ANSI theme for plain output. Pixelblocks live only in the
///header logo and the short footer yarn strip, nowhere else.
pub const GREEN: &str = "\x1b[32m";
pub const BRIGHT: &str = "\x1b[92m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const RESET: &str = "\x1b[0m";

// short accent kept in header/footer only
pub const ACCENT: &str = "\u{2593}\u{2592}\u{2591}";
pub const TAIL: &str = "\u{2591}\u{2592}\u{2593}";

///Wraps text in the main green tone.
pub fn green(s: &str) -> String {
    format!("{GREEN}{s}{RESET}")
}

///Wraps text in bright green for titles and purrfect states.
pub fn bright(s: &str) -> String {
    format!("{BRIGHT}{BOLD}{s}{RESET}")
}

///Short footer yarn strip, the only full-dither remnant.
pub fn yarn_strip(len: usize) -> String {
    let ramp = ["\u{2591}", "\u{2592}", "\u{2593}", "\u{2588}"];
    let mut out = String::new();
    for i in 0..len.min(16) {
        out.push_str(ramp[i % ramp.len()]);
    }
    format!("{GREEN}{out}{RESET}")
}

///Plain divider without pixelblocks.
pub fn divider(width: usize) -> String {
    "─".repeat(width.max(8))
}

///Short logo mark with a small pixel accent and cat face.
pub fn logo() -> String {
    format!("{BRIGHT}{ACCENT} lyx-meow (=^\u{FF65}\u{03C9}\u{FF65}^=){RESET}")
}

///Plain logo without ANSI, for ratatui titles.
pub fn logo_plain() -> String {
    format!("{ACCENT} lyx-meow (=^\u{FF65}\u{03C9}\u{FF65}^=)")
}

///Main green border style for ratatui panes.
pub fn border_style() -> Style {
    Style::default().fg(Color::Green)
}

///Bright title style for ratatui headers.
pub fn title_style() -> Style {
    Style::default()
        .fg(Color::LightGreen)
        .add_modifier(Modifier::BOLD)
}

///Green ok style for purrfect states.
pub fn ok_style() -> Style {
    Style::default()
        .fg(Color::LightGreen)
        .add_modifier(Modifier::BOLD)
}

///Red fail style for hiss states.
pub fn fail_style() -> Style {
    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
}

///Dim style for hints and secondary text.
pub fn dim_style() -> Style {
    Style::default().fg(Color::Gray).add_modifier(Modifier::DIM)
}

#[cfg(test)]
mod tests {
    use super::{divider, yarn_strip};

    #[test]
    fn divider_has_no_pixelblocks() {
        let bar = divider(10);
        for block in ["\u{2591}", "\u{2592}", "\u{2593}", "\u{2588}"] {
            assert!(!bar.contains(block));
        }
    }

    #[test]
    fn yarn_strip_stays_short() {
        let strip = yarn_strip(64);
        let blocks = strip
            .chars()
            .filter(|c| matches!(c, '░' | '▒' | '▓' | '█'))
            .count();
        assert!(blocks <= 16);
    }
}
