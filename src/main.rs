// SPDX-License-Identifier: GPL-3.0-only
use lyx_meow::{App, Args, Command, fuzz, register, runner::compile_path, theme, tui};

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
        Command::Register => return register_mlyx(),
        Command::Unregister => return unregister_mlyx(),
        Command::Fuzz => return fuzz::run(&args),
        _ => {}
    }

    // interactive ratatui only when asked, one-shot pounce by default
    if args.tui {
        let mut app = App::new(args.file.clone(), args.std.clone());
        tui::run_tui(&mut app)?;
        println!("purrr… nap time. bye! (=^\u{FF65}\u{03C9}\u{FF65}^=)");
        return Ok(());
    }
    run_once(&args)
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

// registers the mlyx shortcut for the current user
fn register_mlyx() -> color_eyre::Result<()> {
    let out = register::register()?;
    println!(
        "{} mlyx registered! (=^\u{FF65}\u{03C9}\u{FF65}^=)",
        theme::bright("✓ purrfect")
    );
    println!("  shortcut: {}", out.shortcut);
    println!("  shim: {}", out.shim);
    println!("  target: {}", out.target);
    if out.added_to_path {
        println!("  PATH: added, open a fresh terminal and meow away");
    } else {
        println!("  PATH: already purring, open a fresh terminal if needed");
    }
    Ok(())
}

// unregisters the mlyx shortcut
fn unregister_mlyx() -> color_eyre::Result<()> {
    let out = register::unregister()?;
    if out.removed_shim {
        println!("{} mlyx shim removed ({})", theme::green("✓"), out.shim);
    } else {
        println!(
            "{} no mlyx shim found, nothing to unsheathe",
            theme::green("=")
        );
    }
    if out.removed_from_path {
        println!("  PATH: entry removed, the trail goes cold");
    }
    println!("purr… mlyx unregistered. bye!");
    Ok(())
}

fn print_help() {
    println!("{} — pounce on slynx files", theme::logo());
    println!("{}", Args::usage());
    println!("one-shot by default · --tui/-t opens the ratatui playground");
    println!("fuzz: lyx-meow fuzz --init (starter config) · lyx-meow fuzz [--cases=N] [--rust]");
    println!("keys: r pounce · i knead IR · t sniff tokens · o new sunbeam · q nap");
    println!("register: lyx-meow --register (mlyx shortcut) · lyx-meow --unregister");
}
