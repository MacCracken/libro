.PHONY: check fmt clippy test audit deny build doc bench clean
check: fmt clippy test audit
fmt:
	cargo fmt --all -- --check
clippy:
	cargo clippy --all-targets -- -D warnings
test:
	cargo test
audit:
	cargo audit
deny:
	cargo deny check
bench:
	./scripts/run-benchmarks.sh
build:
	cargo build --release
doc:
	cargo doc --no-deps
clean:
	cargo clean
