use lyx_meow::{App, Args, Command, runner::compile_path, theme, tui};

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

    // interactive ratatui loop, green on green
    let mut app = App::new(args.file.clone(), args.std.clone());
    tui::run_tui(&mut app)?;
    println!("purrr… nap time. bye! (=^\u{FF65}\u{03C9}\u{FF65}^=)");
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
            "{} {} in {}ms of kneading",
            theme::bright("✓ purrfect"),
            path.display(),
            report.ms
        );
        if args.emit_ir {
            println!("{}", report.ir_text);
        }
    } else {
        eprintln!("{} {}", theme::green("✗ hiss…"), path.display());
        eprintln!("{}", report.error);
    }
    Ok(())
}

fn print_help() {
    println!("{} — pounce on slynx files", theme::logo());
    println!("{}", Args::usage());
    println!("keys: r pounce · i knead IR · t sniff tokens · o new sunbeam · q nap");
}
