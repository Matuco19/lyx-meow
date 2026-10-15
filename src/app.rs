use std::path::PathBuf;

use crate::{
    runner::{RunReport, compile_path, preview_source},
    tui,
};

///Minimal TUI state, kept small for easy reasoning.
pub struct App {
    pub file: Option<PathBuf>,
    pub std: Option<PathBuf>,
    pub show_ir: bool,
    pub report: Option<RunReport>,
    pub source: String,
}

impl App {
    pub fn new(file: Option<PathBuf>, std: Option<PathBuf>) -> Self {
        let source = file
            .as_ref()
            .map(|p| preview_source(p, 4000))
            .unwrap_or_else(|| "(no file — pass a .syx path)".to_string());
        Self {
            file,
            std,
            show_ir: true,
            report: None,
            source,
        }
    }

    ///Runs compile once and refreshes preview.
    pub fn run_once(&mut self) {
        if let Some(path) = self.file.clone() {
            self.source = preview_source(&path, 4000);
            self.report = Some(compile_path(&path, self.std.clone()));
        }
    }

    ///Renders current state to stdout.
    pub fn draw(&self) {
        let width = term_width().unwrap_or(80);
        print!(
            "{}",
            tui::render(self.report.as_ref(), &self.source, self.show_ir, width)
        );
    }

    ///Handles one input line, returns false when the app should exit.
    pub fn handle_line(&mut self, line: &str) -> bool {
        match line.trim() {
            "q" | "quit" | "exit" => false,
            "i" => {
                self.show_ir = !self.show_ir;
                true
            }
            "r" | "run" | "" => {
                self.run_once();
                true
            }
            "t" | "tokens" => {
                self.dump_tokens();
                true
            }
            other if other.starts_with("open ") => {
                let path = PathBuf::from(other[5..].trim());
                self.file = Some(path);
                self.run_once();
                true
            }
            _ => true,
        }
    }

    // shows lexer tokens inside the report slot for quick inspection
    fn dump_tokens(&mut self) {
        let Some(path) = self.file.clone() else {
            return;
        };
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        match slynx::slynx_lexer::Lexer::tokenize(&src) {
            Ok(stream) => {
                let text = stream
                    .stream
                    .iter()
                    .take(60)
                    .map(|t| format!("{t}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.report = Some(RunReport {
                    ok: true,
                    ms: 0,
                    ir_text: text,
                    error: String::new(),
                    file: path,
                });
            }
            Err(e) => {
                self.report = Some(RunReport::fail(path, e.to_string()));
            }
        }
    }
}

// reads COLUMNS env or defaults, avoids extra deps
fn term_width() -> Option<usize> {
    std::env::var("COLUMNS").ok().and_then(|s| s.parse().ok())
}
