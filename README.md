# lyx-meow

> Green TUI runner for [slynx](https://github.com/Slynx-Language/slynx).

Minimal, dependency-light (`slynx` + `color-eyre` + `thiserror`), std-only ANSI UI with green dither pixelblock chrome (`░▒▓█`).

## Install

```bash
cargo build
```

## Use

```bash
# interactive TUI (run once on open, then r/i/t/q)
cargo run -- examples/hello.syx

# one-shot for scripts / CI
cargo run -- run examples/hello.syx --no-tui

# tokens only
cargo run -- tokens examples/hello.syx --no-tui

# custom std path
cargo run -- run examples/hello.syx --std=../slynx-fork/lib/std --no-tui
```

Keys: `r` run · `i` toggle IR · `t` tokens · `open <path>` · `q` quit.

## Layout

- `src/cli.rs` — arg parsing, no deps
- `src/runner.rs` — compile wrapper + timing
- `src/theme.rs` — green ANSI + pixelblocks
- `src/tui.rs` — pure-string frame renderer
- `src/app.rs` — small state machine

## Validate

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```
