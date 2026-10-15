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

#Run the TUI on the hello example without interaction.
run:
	cargo run -- run examples/hello.syx --no-tui
