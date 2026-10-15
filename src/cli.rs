use std::path::PathBuf;

///TUI runner for slynx sources with green pixelblock chrome.
#[derive(Debug, Clone)]
pub struct Args {
    pub command: Command,
    pub file: Option<PathBuf>,
    pub std: Option<PathBuf>,
    pub no_tui: bool,
    pub emit_ir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    Check,
    Tokens,
    Help,
    Version,
}

impl Args {
    ///Parses process args without extra deps to keep the binary minimal.
    pub fn parse(mut raw: impl Iterator<Item = String>) -> Self {
        // skip argv0
        let _ = raw.next();
        let mut out = Self {
            command: Command::Run,
            file: None,
            std: None,
            no_tui: false,
            emit_ir: true,
        };
        for arg in raw {
            match arg.as_str() {
                "run" => out.command = Command::Run,
                "check" => out.command = Command::Check,
                "tokens" => out.command = Command::Tokens,
                "help" | "--help" | "-h" => out.command = Command::Help,
                "version" | "--version" | "-V" => out.command = Command::Version,
                "--no-tui" => out.no_tui = true,
                "--no-ir" => out.emit_ir = false,
                s if s.starts_with("--std=") => {
                    out.std = Some(PathBuf::from(&s[6..]));
                }
                s if s.starts_with('-') => {
                    // ignore unknown flags to stay forgiving
                }
                s => {
                    if out.file.is_none() {
                        out.file = Some(PathBuf::from(s));
                    }
                }
            }
        }
        out
    }

    pub fn usage() -> &'static str {
        "usage: lyx-meow [run|check|tokens] <file.syx> [--std=PATH] [--no-tui] [--no-ir]"
    }
}

#[cfg(test)]
mod tests {
    use super::{Args, Command};

    #[test]
    fn parses_run_with_file() {
        let args = Args::parse(
            ["lyx-meow", "run", "examples/hello.syx"]
                .iter()
                .map(|s| s.to_string()),
        );
        assert!(matches!(args.command, Command::Run));
        assert_eq!(
            args.file.expect("file should parse").to_string_lossy(),
            "examples/hello.syx"
        );
    }

    #[test]
    fn parses_no_tui_flag() {
        let args = Args::parse(
            ["lyx-meow", "check", "a.syx", "--no-tui"]
                .iter()
                .map(|s| s.to_string()),
        );
        assert!(matches!(args.command, Command::Check));
        assert!(args.no_tui);
    }
}
