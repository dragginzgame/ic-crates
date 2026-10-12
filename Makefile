.DEFAULT_GOAL := help
SHARED_TOOLING_ROOT := $(CURDIR)
include make/consumer.mk

PACKAGES = -p ic-crates-protocol -p canister_ic_crates_registry -p canister_ic_crates_blob -p ic-crates-publisher
export CARGO_TARGET_DIR := $(CURDIR)/target
export VALIDATION_LOG_DIR ?= $(CURDIR)/target/validation-runs
SHELLCHECK ?= $(CURDIR)/.tools/ci/bin/shellcheck
ACTIONLINT ?= $(CURDIR)/.tools/ci/bin/actionlint
GITLEAKS ?= $(CURDIR)/.tools/ci/bin/gitleaks

.PHONY: help version tasks deps check test test-protocol test-catalog clippy docs-check \
        metadata-check runtime-check check-pins check-doc-links ci msrv-check \
        package-check publish-dry-run publish check-release-commands test-release \
        release-version release-preflight release-verify release-prepare-version \
        release-prepared-check release-files release-commit-check release-committed-check \
        release-tagged-check release-push-check install-validation-tools lint check-secrets

help:
	@echo 'deps / install-tools / tools-check: prepare dependencies and the common toolset'
	@echo 'fmt / fmt-check / install-hooks: shared formatting and pre-commit activation'
	@echo 'check / test / clippy / docs-check: focused Rust development checks'
	@echo 'ci: complete native delivery gate with retained validation logs'
	@echo 'package-check / publish-dry-run: verify the protocol archive before publication'
	@echo 'release-patch / release-minor / release-major: user-operated shared release workflow'
	@echo 'publish: user-operated crates.io publication of the exact tagged protocol package'
	@echo 'tasks / cloc: shared maintenance catalog and workspace Rust LOC'

version release-version:
	@bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml

tasks:
	@cat tasks/README.md

deps:
	+cargo fetch --locked

metadata-check:
	+cargo metadata --offline --locked --no-deps --format-version 1 > /dev/null
	cmp LICENSE crates/ic-crates-protocol/LICENSE

runtime-check:
	+bash scripts/ci/check-runtime-identities.sh

check:
	+cargo check --offline --locked $(PACKAGES) --all-targets

test:
	+cargo test --offline --locked -p ic-crates-protocol -p canister_ic_crates_registry -p ic-crates-publisher

test-protocol:
	+cargo test --offline --locked -p ic-crates-protocol

test-catalog:
	+cargo test --offline --locked -p canister_ic_crates_registry --lib

clippy:
	+cargo clippy --offline --locked $(PACKAGES) --all-targets --no-deps -- -D warnings

docs-check:
	+RUSTDOCFLAGS='-D warnings' cargo doc --offline --locked $(PACKAGES) --no-deps

check-pins:
	+bash scripts/ci/check-dependency-pins.sh --consumer "$(CURDIR)" --cargo-inheritance

check-doc-links:
	perl scripts/ci/check-documentation-links.pl --root "$(CURDIR)" \
		README.md AGENTS.md CHANGELOG.md docs/status/current.md docs/development.md \
		docs/publishing.md docs/cargo-publish.md crates/ic-crates-protocol/README.md DRAGGINZGAME.md \
		rules/*.md tasks/*.md audits/*.md docs/principles/*.md docs/supported-hosts.md \
		docs/local-setup.md docs/ic-tools.md docs/make-interface.md docs/releases.md \
		docs/consuming-snapshots.md docs/verification-helpers.md docs/tag-maintenance.md docs/provenance.md

msrv-check:
	+cargo +1.88.0 check --offline --locked -p ic-crates-protocol --all-targets
	+cargo +1.91.0 check --offline --locked $(PACKAGES) --all-targets

package-check publish-dry-run:
	+cargo package --offline --locked -p ic-crates-protocol --allow-dirty

ci:
	+@bash scripts/ci/run-validation-targets.sh --fail-fast \
		shared-tooling-check tools-check metadata-check runtime-check check-pins \
		check-doc-links fmt-check check test clippy docs-check msrv-check \
		package-check check-release-commands test-release

check-release-commands:
	@bash scripts/ci/check-release-commands.sh "$(CURDIR)" \
		make/consumer.mk make/tools.mk make/rust-format.mk make/release.mk \
		make/execution.mk scripts/ci/check-make-execution.sh

test-release:
	@bash scripts/release/test-release.sh

release-preflight release-verify release-prepare-version release-prepared-check \
release-files release-commit-check release-committed-check release-tagged-check release-push-check:
	+@bash scripts/release/release.sh "$(@:release-%=%)"

publish:
	+@bash scripts/release/publish.sh

install-validation-tools:
	@bash -ec 'test "$$(uname -s):$$(uname -m)" = Linux:x86_64; \
		. ci/tool-versions.env; \
		bash scripts/ci/install-shellcheck.sh --version "$$SHARED_TOOLING_SHELLCHECK_VERSION" \
			--sha256 "$$SHARED_TOOLING_SHELLCHECK_SHA256_LINUX_X86_64" --install-dir "$(CURDIR)/.tools/ci/bin"; \
		bash scripts/ci/install-actionlint.sh --version "$$SHARED_TOOLING_ACTIONLINT_VERSION" \
			--sha256 "$$SHARED_TOOLING_ACTIONLINT_SHA256_LINUX_AMD64" --install-dir "$(CURDIR)/.tools/ci/bin"; \
		bash scripts/ci/install-gitleaks.sh --version "$$SHARED_TOOLING_GITLEAKS_VERSION" \
			--sha256 "$$SHARED_TOOLING_GITLEAKS_SHA256_LINUX_X64" --install-dir "$(CURDIR)/.tools/ci/bin"'

lint:
	"$(SHELLCHECK)" scripts/release/*.sh scripts/ci/check-runtime-identities.sh .githooks/pre-commit
	"$(ACTIONLINT)" .github/workflows/ci.yml

check-secrets:
	"$(GITLEAKS)" git --redact=100 --no-banner --no-color --log-level error "$(CURDIR)"
