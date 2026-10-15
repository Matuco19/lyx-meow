# lyx-meow

> TUI runner for [slynx](https://github.com/Slynx-Language/slynx). Pounce on `.syx` files, knead the IR, nap when done. `(=^･ω･^=)`

## Install

```bash
cargo build
```

## Use

```bash
# interactive ratatui TUI (pounces once on open)
cargo run -- examples/hello.syx

# one-shot for scripts / CI
cargo run -- run examples/hello.syx --no-tui

# one-shot for scripts / CI
cargo run -- run examples/hello.syx -n

# tokens only (sniffing)
cargo run -- tokens examples/hello.syx --no-tui

# custom std path
cargo run -- run examples/hello.syx --std=../slynx-fork/lib/std --no-tui
```

Keys: `r` pounce (run) · `i` knead IR on/off · `t` sniff tokens · `o` new sunbeam (type a path) · `↑↓/jk` scroll · `q` nap (quit).

## Layout

- `src/cli.rs` — arg parsing, no deps
- `src/runner.rs` — compile wrapper + timing
- `src/theme.rs` — green styles + tiny pixel accent
- `src/tui.rs` — ratatui event loop + panes
- `src/app.rs` — small state machine

## Validate

```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```
