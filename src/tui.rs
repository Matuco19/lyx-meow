use crate::{
    runner::RunReport,
    theme::{self, BOLD, DIM, GREEN, RESET},
};

///Clears the screen and homes the cursor with ANSI only.
pub fn clear() -> String {
    "\x1b[2J\x1b[H".to_string()
}

///Renders one full frame for the report.
pub fn render(report: Option<&RunReport>, source: &str, show_ir: bool, width: usize) -> String {
    let w = width.clamp(40, 120);
    let mut out = String::new();
    out.push_str(&clear());
    out.push_str(&theme::dither_bar(w, 0));
    out.push('\n');
    out.push_str(&format!("{}  {}\n", theme::logo(), hint_line()));
    out.push_str(&theme::divider(w));
    out.push('\n');
    match report {
        None => {
            out.push_str(&theme::green("  ░ idle — press r to run, q to quit"));
            out.push('\n');
        }
        Some(r) => {
            out.push_str(&status_line(r));
            out.push('\n');
            out.push_str(&theme::divider(w));
            out.push('\n');
            if r.ok && show_ir && !r.ir_text.is_empty() {
                out.push_str(&clip(&r.ir_text, 30));
                out.push('\n');
                out.push_str(&theme::divider(w));
                out.push('\n');
            } else if !r.ok {
                out.push_str(&clip(&r.error, 24));
                out.push('\n');
                out.push_str(&theme::divider(w));
                out.push('\n');
            }
        }
    }
    out.push_str(&format!("{DIM}── source ──{RESET}\n"));
    out.push_str(&clip(source, 14));
    out.push('\n');
    out.push_str(&theme::dither_bar(w, 2));
    out.push('\n');
    out.push_str(&format!(
        "{GREEN}[r] run  [i] ir on/off  [t] tokens  [q] quit{RESET}\n"
    ));
    out
}

///One-line status with timing.
fn status_line(r: &RunReport) -> String {
    if r.ok {
        format!(
            "  {} {} {}in {}ms  {}▓{}",
            theme::bright("✓ OK"),
            r.file.to_string_lossy(),
            DIM,
            r.ms,
            GREEN,
            RESET
        )
    } else {
        format!(
            "  {BOLD}\x1b[31m✗ FAIL{RESET} {} {DIM}in {}ms{RESET}",
            r.file.to_string_lossy(),
            r.ms
        )
    }
}

///Keeps output short for small terminals.
fn clip(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().take(max_lines).collect();
    let mut out = lines.join("\n");
    if s.lines().count() > max_lines {
        out.push_str(&format!("\n{DIM}… truncated{RESET}"));
    }
    if out.is_empty() {
        out.push_str(&format!("{DIM}(empty){RESET}"));
    }
    out
}

fn hint_line() -> String {
    format!("{DIM}slynx runner · green TUI{RESET}")
}
