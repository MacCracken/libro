.PHONY: check fmt clippy test test-all audit deny bench coverage doc build clean

check: fmt clippy test audit

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

test-all:
	cargo test --features sqlite,signing,streaming

audit:
	cargo audit

deny:
	cargo deny check

bench:
	./scripts/run-benchmarks.sh

coverage:
	cargo tarpaulin --features sqlite,signing,streaming --skip-clean

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

build:
	cargo build --release

clean:
	cargo clean
