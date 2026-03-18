.PHONY: all build release clean test test-host test-kernel

build:                   ## Build kernel (debug)
	cargo build

release:                 ## Build kernel (release, optimized)
	cargo build --release

test: test-host          ## Run all tests

test-host:               ## Run host-side unit tests
	cargo test --workspace --target x86_64-unknown-linux-gnu

clippy:                  ## Run Clippy linter
	cargo clippy -- -D warnings

fmt:                     ## Format all code
	cargo fmt --all

fmt-check:               ## Check formatting
	cargo fmt --all -- --check

clean:                   ## Remove build artifacts
	cargo clean
	rm -rf build/

help:                    ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' Makefile | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
