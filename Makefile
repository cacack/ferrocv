.PHONY: help \
        preflight fmt-check fmt clippy test deny audit typos \
        build build-release check clean doc install \
        install-tools

# CI-parity targets: keep command strings in sync with
# .github/workflows/ci.yml so local runs match CI byte-for-byte.
# If CI changes, update here too.

help: ## Show available targets
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

# --- CI parity ---------------------------------------------------------------

preflight: fmt-check clippy test deny audit typos ## Run all CI checks locally

fmt-check: ## cargo fmt --check (CI parity)
	cargo fmt --all -- --check

fmt: ## Apply cargo fmt
	cargo fmt --all

clippy: ## cargo clippy with -D warnings (CI parity)
	cargo clippy --all-targets --all-features -- -D warnings

test: ## cargo test (CI parity)
	cargo test --all-features --workspace

deny: ## cargo-deny check (CI parity)
	cargo deny --all-features check

audit: ## cargo-audit (CI parity)
	cargo audit

typos: ## typos check (CI parity)
	typos

# --- Build & dev -------------------------------------------------------------

build: ## cargo build (debug)
	cargo build

build-release: ## cargo build --release
	cargo build --release

check: ## cargo check (fast type-check, no lints)
	cargo check --all-targets --all-features

clean: ## cargo clean
	cargo clean

doc: ## cargo doc (this crate only, opens in browser)
	cargo doc --no-deps --open

install: ## Install ferrocv binary from this checkout
	cargo install --locked --path .

# --- Tooling -----------------------------------------------------------------

install-tools: ## Install cargo-deny, cargo-audit, typos-cli
	cargo install --locked cargo-deny cargo-audit typos-cli
