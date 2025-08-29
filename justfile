default: lint build test

test:
  cargo test --all-features -- --show-output

build:
  cargo build
  cargo check

fmt:
  cargo fmt --all

lint:
  cargo clippy --all -- -D warnings

# Run more strict linter
pedantic:
  cargo clippy -- -W clippy::pedantic

audit:
  cargo audit

# Install cargo tools used in package maitanance
init_dev:
  cargo install git-cliff
  cargo install cargo-bloat
  cargo install cargo-audit

examples:
  cd examples/simple && cargo build
  cd examples/sbus-esp32c6 && cargo build --release

