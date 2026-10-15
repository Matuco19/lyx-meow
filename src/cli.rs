// SPDX-License-Identifier: GPL-3.0-only
use std::path::PathBuf;

///One-shot runner for slynx sources by default, green with a small pixel accent.
#[derive(Debug, Clone)]
pub struct Args {
    pub command: Command,
    pub file: Option<PathBuf>,
    pub std: Option<PathBuf>,
    pub tui: bool,
    pub emit_ir: bool,
    pub fuzz_config: Option<PathBuf>,
    pub fuzz_cases: Option<u64>,
    pub fuzz_seed: Option<u64>,
    pub fuzz_out: Option<PathBuf>,
    pub fuzz_init: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    Check,
    Tokens,
    Fuzz,
    Register,
    Unregister,
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
            tui: false,
            emit_ir: true,
            fuzz_config: None,
            fuzz_cases: None,
            fuzz_seed: None,
            fuzz_out: None,
            fuzz_init: false,
        };
        for arg in raw {
            match arg.as_str() {
                "run" => out.command = Command::Run,
                "check" => out.command = Command::Check,
                "tokens" => out.command = Command::Tokens,
                "fuzz" => out.command = Command::Fuzz,
                "register" | "--register" => out.command = Command::Register,
                "unregister" | "--unregister" => out.command = Command::Unregister,
                "help" | "--help" | "-h" => out.command = Command::Help,
                "version" | "--version" | "-V" => out.command = Command::Version,
                "--tui" | "-t" => out.tui = true,
                "--no-ir" => out.emit_ir = false,
                "--init" => out.fuzz_init = true,
                s if s.starts_with("--std=") => {
                    out.std = Some(PathBuf::from(&s[6..]));
                }
                s if s.starts_with("--config=") => {
                    out.fuzz_config = Some(PathBuf::from(&s[9..]));
                }
                s if s.starts_with("--cases=") => {
                    out.fuzz_cases = s[8..].parse().ok();
                }
                s if s.starts_with("--seed=") => {
                    out.fuzz_seed = s[7..].parse().ok();
                }
                s if s.starts_with("--out=") => {
                    out.fuzz_out = Some(PathBuf::from(&s[6..]));
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
        "usage: lyx-meow [run|check|tokens] <file.syx> [--std=PATH] [--tui|-t] [--no-ir]\n       lyx-meow fuzz [--config=PATH] [--cases=N] [--seed=N] [--out=DIR] [--init]\n       lyx-meow [register|unregister]"
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
    fn one_shot_is_default() {
        let args = Args::parse(["lyx-meow", "a.syx"].iter().map(|s| s.to_string()));
        assert!(!args.tui);
    }

    #[test]
    fn parses_tui_flag() {
        let args = Args::parse(
            ["lyx-meow", "run", "a.syx", "--tui"]
                .iter()
                .map(|s| s.to_string()),
        );
        assert!(matches!(args.command, Command::Run));
        assert!(args.tui);
    }

    #[test]
    fn parses_tui_short_flag() {
        let args = Args::parse(["lyx-meow", "a.syx", "-t"].iter().map(|s| s.to_string()));
        assert!(args.tui);
    }

    #[test]
    fn parses_register_commands() {
        let args = Args::parse(["lyx-meow", "--register"].iter().map(|s| s.to_string()));
        assert!(matches!(args.command, Command::Register));
        let args = Args::parse(["lyx-meow", "unregister"].iter().map(|s| s.to_string()));
        assert!(matches!(args.command, Command::Unregister));
    }

    #[test]
    fn parses_fuzz_overrides() {
        let args = Args::parse(
            [
                "lyx-meow",
                "fuzz",
                "--cases=9",
                "--seed=3",
                "--out=out",
                "--init",
            ]
            .iter()
            .map(|s| s.to_string()),
        );
        assert!(matches!(args.command, Command::Fuzz));
        assert_eq!(args.fuzz_cases, Some(9));
        assert_eq!(args.fuzz_seed, Some(3));
        assert!(args.fuzz_init);
        assert_eq!(
            args.fuzz_out.expect("out should parse").to_string_lossy(),
            "out"
        );
    }
}
