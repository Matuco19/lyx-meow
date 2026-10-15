use std::io::{BufRead, Write};

use lyx_meow::{App, Args, Command, runner::compile_path, theme};

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse(std::env::args());

    match args.command {
        Command::Help => {
            print_help();
            return Ok(());
        }
        Command::Version => {
            println!("lyx-meow {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }

    // one-shot mode for scripts and CI
    if args.no_tui {
        return run_once(&args);
    }

    // interactive green TUI loop
    let mut app = App::new(args.file.clone(), args.std.clone());
    app.run_once();
    app.draw();
    let stdin = std::io::stdin();
    print!("lyx> ");
    let _ = std::io::stdout().flush();
    for line in stdin.lock().lines() {
        let line = line.unwrap_or_default();
        if !app.handle_line(&line) {
            break;
        }
        app.draw();
        print!("lyx> ");
        let _ = std::io::stdout().flush();
    }
    println!("{}", theme::dither_bar(40, 1));
    println!("{} bye", theme::logo());
    Ok(())
}

// compiles once and prints themed output without looping
fn run_once(args: &Args) -> color_eyre::Result<()> {
    let Some(path) = args.file.clone() else {
        eprintln!("{}", Args::usage());
        return Ok(());
    };
    if args.command == Command::Tokens {
        let src = std::fs::read_to_string(&path)?;
        let stream = slynx::slynx_lexer::Lexer::tokenize(&src)
            .map_err(|e| color_eyre::eyre::eyre!("{e}"))?;
        for token in stream.stream {
            println!("{token}");
        }
        return Ok(());
    }
    let report = compile_path(&path, args.std.clone());
    if report.ok {
        println!(
            "{} {} in {}ms",
            theme::bright("✓ OK"),
            path.display(),
            report.ms
        );
        if args.emit_ir {
            println!("{}", report.ir_text);
        }
    } else {
        eprintln!("{} {}", theme::green("✗ FAIL"), path.display());
        eprintln!("{}", report.error);
    }
    Ok(())
}

fn print_help() {
    println!("{} — green slynx runner", theme::logo());
    println!("{}", Args::usage());
    println!("keys: r run · i toggle ir · t tokens · open <path> · q quit");
}
