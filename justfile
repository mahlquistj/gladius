export RUST_BACKTRACE := "1"

# Print this list of scripts
list:
    @just --list

# Run tests
test:
    @cargo nextest run --hide-progress-bar --failure-output final {{ARGS}}

# Run linters and formatting
lint:
    @cargo fmt -- --check --color always
    @cargo clippy --all-targets --all-features -- -D warnings

build ARGS="":
    @cargo build {{ARGS}}

# Run benchmarks
bench ARGS="":
    @cargo bench {{ARGS}}

# Publish crate
publish:
    @cargo publish
