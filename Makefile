.PHONY: test
#Test the project and print output to the terminal.
test:
	cargo test --verbose

#Checks if the runner is working properly, according to CI/CD
check:
	cargo test --verbose
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo build --verbose

#Run the hello example one-shot (no TUI by default).
run:
	cargo run -- run examples/hello.syx

#Hunt hairballs in the slynx pipeline with the default config.
fuzz:
	cargo run -- fuzz --cases=200

#Torch the slynx rust sources as lexer-torture seeds.
fuzz-rust:
	cargo run -- fuzz --rust --cases=500
