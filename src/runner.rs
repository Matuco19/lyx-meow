use std::{
    path::{Path, PathBuf},
    time::Instant,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("file not found: {0}")]
    MissingFile(String),
    #[error("compile failed:\n{0}")]
    Compile(String),
    #[error("io: {0}")]
    Io(String),
}

///Single compile result with timing for display.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub ok: bool,
    pub ms: u128,
    pub ir_text: String,
    pub error: String,
    pub file: PathBuf,
}

impl RunReport {
    pub fn fail(file: PathBuf, error: String) -> Self {
        Self {
            ok: false,
            ms: 0,
            ir_text: String::new(),
            error,
            file,
        }
    }
}

///Compiles the file at `path` into SIR text, measuring elapsed time.
pub fn compile_path(path: &Path, std: Option<PathBuf>) -> RunReport {
    // friendly error before touching the compiler
    if !path.exists() {
        return RunReport::fail(
            path.to_path_buf(),
            RunnerError::MissingFile(path.to_string_lossy().to_string()).to_string(),
        );
    }
    let start = Instant::now();
    match slynx::compile_to_ir(path.to_path_buf(), std) {
        Ok(ir) => RunReport {
            ok: true,
            ms: start.elapsed().as_millis(),
            ir_text: ir.format_sir(),
            error: String::new(),
            file: path.to_path_buf(),
        },
        Err(e) => RunReport {
            ok: false,
            ms: start.elapsed().as_millis(),
            ir_text: String::new(),
            // color-eyre already formats with context
            error: format!("{e:?}"),
            file: path.to_path_buf(),
        },
    }
}

///Reads up to `limit` bytes of source for preview.
pub fn preview_source(path: &Path, limit: usize) -> String {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            if s.len() <= limit {
                s
            } else {
                format!("{}\n… [truncated]", &s[..limit])
            }
        }
        Err(e) => format!("<unreadable: {e}>"),
    }
}
