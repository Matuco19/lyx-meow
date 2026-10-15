///Green ANSI theme with dither pixelblock chrome.
pub const GREEN: &str = "\x1b[32m";
pub const BRIGHT: &str = "\x1b[92m";
pub const DIM: &str = "\x1b[2m";
pub const BOLD: &str = "\x1b[1m";
pub const RESET: &str = "\x1b[0m";

// dither ramp used for borders and meters, dark to dense
pub const DITHER: [&str; 4] = ["\u{2591}", "\u{2592}", "\u{2593}", "\u{2588}"];

///Wraps text in the main green tone.
pub fn green(s: &str) -> String {
    format!("{GREEN}{s}{RESET}")
}

///Wraps text in bright green for titles and ok states.
pub fn bright(s: &str) -> String {
    format!("{BRIGHT}{BOLD}{s}{RESET}")
}

///Builds a full-width dither bar like `▓▓▒▒░░` cycled.
pub fn dither_bar(width: usize, offset: usize) -> String {
    let mut out = String::with_capacity(width * 3);
    for i in 0..width {
        out.push_str(DITHER[(i + offset) % DITHER.len()]);
    }
    format!("{GREEN}{out}{RESET}")
}

///Builds a thin divider with pixelblocks on both ends.
pub fn divider(width: usize) -> String {
    let w = width.max(8);
    let inner = "─".repeat(w - 4);
    format!("{}▒{inner}▒{}", GREEN, RESET)
}

///Short logo mark used in header and help.
pub fn logo() -> String {
    format!("{BRIGHT}▓▒░ lyx-meow ░▒▓{RESET}")
}

#[cfg(test)]
mod tests {
    use super::dither_bar;

    #[test]
    fn dither_bar_cycles_ramp() {
        let bar = dither_bar(8, 0);
        assert!(bar.contains("\u{2591}"));
        assert!(bar.contains("\u{2588}"));
    }
}
