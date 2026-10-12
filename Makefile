.PHONY: fmt fmt-check check test-protocol test-catalog clippy

PACKAGES = -p ic-crates-protocol -p canister_ic_crates_registry -p canister_ic_crates_blob

fmt:
	cargo fmt $(PACKAGES)

fmt-check:
	cargo fmt $(PACKAGES) -- --check

check:
	cargo check --offline --locked $(PACKAGES)

test-protocol:
	cargo test --offline --locked -p ic-crates-protocol

test-catalog:
	cargo test --offline --locked -p canister_ic_crates_registry --lib

clippy:
	cargo clippy --offline --locked $(PACKAGES) --all-targets --no-deps -- -D warnings
