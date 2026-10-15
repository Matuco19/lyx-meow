// SPDX-License-Identifier: GPL-3.0-only
use std::path::PathBuf;

use crate::runner::{RunReport, compile_path, preview_source};

///Small TUI state, kept minimal for easy reasoning.
pub struct App {
    pub file: Option<PathBuf>,
    pub std: Option<PathBuf>,
    pub show_ir: bool,
    pub report: Option<RunReport>,
    pub source: String,
    pub scroll: u16,
    pub input_mode: bool,
    pub input: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(file: Option<PathBuf>, std: Option<PathBuf>) -> Self {
        let source = file
            .as_ref()
            .map(|p| preview_source(p, 4000))
            .unwrap_or_else(|| "(no sunbeam yet — pass a .syx path to pounce on)".to_string());
        Self {
            file,
            std,
            show_ir: true,
            report: None,
            source,
            scroll: 0,
            input_mode: false,
            input: String::new(),
            should_quit: false,
        }
    }

    ///Runs compile once and refreshes preview.
    pub fn run_once(&mut self) {
        self.scroll = 0;
        if let Some(path) = self.file.clone() {
            self.source = preview_source(&path, 4000);
            self.report = Some(compile_path(&path, self.std.clone()));
        }
    }

    ///Flips IR kneading on and off.
    pub fn toggle_ir(&mut self) {
        self.show_ir = !self.show_ir;
        self.scroll = 0;
    }

    ///Scrolls the yarn ball pane.
    pub fn scroll_by(&mut self, delta: i16) {
        let next = self.scroll as i16 + delta;
        self.scroll = next.max(0) as u16;
    }

    ///Enters sunbeam path input.
    pub fn enter_input(&mut self) {
        self.input_mode = true;
        self.input.clear();
    }

    ///Cancels sunbeam path input.
    pub fn cancel_input(&mut self) {
        self.input_mode = false;
        self.input.clear();
    }

    ///Submits sunbeam path input and pounces.
    pub fn submit_input(&mut self) {
        let path = PathBuf::from(self.input.trim());
        self.input_mode = false;
        self.input.clear();
        if !path.as_os_str().is_empty() {
            self.file = Some(path);
            self.run_once();
        }
    }

    ///Status line in cat words.
    pub fn status_text(&self) -> String {
        match &self.report {
            None => "😺 curling up… press r to pounce, q to nap".to_string(),
            Some(r) if r.ok => format!(
                "✓ purrfect · {} · {}ms of kneading",
                r.file.to_string_lossy(),
                r.ms
            ),
            Some(r) => format!(
                "✗ hiss… {} · {}ms of hissing",
                r.file.to_string_lossy(),
                r.ms
            ),
        }
    }

    ///Text shown in the yarn ball pane.
    pub fn output_text(&self) -> String {
        match &self.report {
            None => "😺 no yarn ball yet — pounce with r".to_string(),
            Some(r) if !r.ok => r.error.clone(),
            Some(r) if self.show_ir && !r.ir_text.is_empty() => r.ir_text.clone(),
            Some(_) => "😺 IR is curled up (press i to knead it back out)".to_string(),
        }
    }

    // shows lexer tokens inside the report slot for quick sniffing
    pub fn dump_tokens(&mut self) {
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
        self.scroll = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    #[test]
    fn status_uses_cat_words() {
        let app = App::new(None, None);
        let status = app.status_text();
        assert!(status.contains("pounce") || status.contains("curling"));
    }
}
