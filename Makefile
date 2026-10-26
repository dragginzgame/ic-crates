.DEFAULT_GOAL := help
SHARED_TOOLING_ROOT := $(CURDIR)
IC_TOOLS := didc ic-wasm icp quill wasm-opt
LOCAL_TOOL_INSTALL_TARGETS := install-ic-tools install-app-tools
LOCAL_TOOL_CHECK_TARGETS := ic-tools-check app-tools-check
include make/consumer.mk

PACKAGES = -p ic-crates-protocol -p canister_ic_crates_registry -p canister_ic_crates_blob -p ic-crates -p ic-crates-tests
export CARGO_TARGET_DIR := $(CURDIR)/target
export VALIDATION_LOG_DIR ?= $(CURDIR)/target/validation-runs
SHELLCHECK ?= $(CURDIR)/.tools/ci/bin/shellcheck
ACTIONLINT ?= $(CURDIR)/.tools/ci/bin/actionlint
GITLEAKS ?= $(CURDIR)/.tools/ci/bin/gitleaks

.PHONY: help version tasks deps check test test-protocol test-catalog clippy docs-check \
        metadata-check runtime-check check-pins check-doc-links ci msrv-check \
        package-check publish-dry-run publish check-release-commands test-release test-publish \
        release-version release-preflight release-verify release-prepare-version \
        release-prepared-check release-files release-commit-check release-committed-check \
        release-tagged-check release-push-check install-validation-tools lint check-secrets
.PHONY: install-app-tools app-tools-check managed-build test-ic test-blob-worker deployment-deps-check external-ic-check

help:
	@echo 'deps / install-tools / tools-check: prepare dependencies and the common toolset'
	@echo 'fmt / fmt-check / install-hooks: shared formatting and pre-commit activation'
	@echo 'check / test / clippy / docs-check: focused Rust development checks'
	@echo 'managed-build / test-ic: Canic artifacts and actual registry/blob PocketIC qualification'
	@echo 'ci: complete native and managed IC gate with retained validation logs'
	@echo 'package-check / publish-dry-run: verify protocol and CLI archives before publication'
	@echo 'release-patch / release-minor / release-major: user-operated shared release workflow'
	@echo 'publish: user-operated crates.io publication of the exact tagged protocol, then CLI'
	@echo 'tasks / cloc: shared maintenance catalog and workspace Rust LOC'

version release-version:
	@bash scripts/ci/read-cargo-workspace-version.sh --stable Cargo.toml

tasks:
	@cat tasks/README.md

deps:
	+cargo fetch --locked

metadata-check:
	+cargo metadata --offline --locked --no-deps --format-version 1 | jq -e \
		'([.packages[] | select(.publish != []) | .name] | sort) == ["ic-crates", "ic-crates-protocol"] and any(.packages[]; .name == "ic-crates" and any(.targets[]; .kind == ["bin"] and .name == "ic-crates"))' > /dev/null
	cmp LICENSE crates/ic-crates-protocol/LICENSE
	cmp LICENSE apps/ic_crates/publisher/LICENSE

runtime-check:
	+bash scripts/ci/check-runtime-identities.sh

check:
	+cargo check --offline --locked $(PACKAGES) --all-targets

test:
	+cargo test --offline --locked -p ic-crates-protocol -p canister_ic_crates_registry -p ic-crates -p ic-crates-tests

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

deployment-deps-check:
	+perl scripts/ci/check-cargo-graph.pl --consumer "$(CURDIR)" \
		--package canister_ic_crates_registry --package canister_ic_crates_blob \
		--target wasm32-unknown-unknown --forbid canic-host --forbid ic-testkit --explain
	+@host="$$(rustc -vV | sed -n 's/^host: //p')"; \
		perl scripts/ci/check-cargo-graph.pl --consumer "$(CURDIR)" --package ic-crates \
		--target "$$host" --forbid canic-host --forbid ic-testkit --explain

external-ic-check:
	+bash scripts/ci/check-cargo-dependency-boundaries.sh --consumer "$(CURDIR)" --external-ic

check-doc-links:
	perl scripts/ci/check-documentation-links.pl --root "$(CURDIR)" \
		README.md AGENTS.md CHANGELOG.md docs/status/current.md docs/development.md \
		docs/publishing.md docs/cargo-publish.md docs/blob-worker.md crates/ic-crates-protocol/README.md \
		crates/ic-crates-protocol/UPSTREAM.md \
		apps/ic_crates/publisher/README.md DRAGGINZGAME.md \
		rules/*.md tasks/*.md audits/*.md docs/principles/*.md docs/supported-hosts.md \
		docs/local-setup.md docs/ic-tools.md docs/make-interface.md docs/releases.md \
		docs/consuming-snapshots.md docs/verification-helpers.md docs/tag-maintenance.md docs/provenance.md \
		docs/cargo-graph-checks.md docs/cargo-publication.md docs/rust-toolchains.md \
		docs/tool-catalog-bindings.md docs/ci-change-selection.md

msrv-check:
	+cargo +1.91.0 check --offline --locked $(PACKAGES) --all-targets

package-check publish-dry-run:
	+cargo package --locked --registry crates-io -p ic-crates-protocol -p ic-crates --allow-dirty

ci:
	+@bash scripts/ci/run-validation-targets.sh --fail-fast \
		shared-tooling-check tools-check metadata-check runtime-check check-pins \
		deployment-deps-check external-ic-check check-doc-links fmt-check clippy check test docs-check msrv-check \
		package-check check-release-commands test-release test-publish managed-build test-ic

install-app-tools:
	+bash scripts/ci/managed-ic.sh install

app-tools-check:
	+bash scripts/ci/managed-ic.sh check

managed-build: deployment-deps-check external-ic-check
	+bash scripts/ci/managed-ic.sh build

test-blob-worker:
	@test -n "$(IC_CRATES_BLOB_TEST_CONFIGURATION)" || { echo "Select IC_CRATES_BLOB_TEST_CONFIGURATION; see docs/blob-worker.md" >&2; exit 1; }
	+cargo test --offline --locked -p ic-crates real_sdk_preflight -- --ignored --nocapture

test-ic:
	+bash scripts/ci/managed-ic.sh test

check-release-commands:
	@bash scripts/ci/check-release-commands.sh "$(CURDIR)" \
		make/consumer.mk make/tools.mk make/rust-format.mk make/release.mk \
		make/execution.mk scripts/ci/check-make-execution.sh

test-release:
	@bash scripts/release/test-release.sh

test-publish:
	@bash scripts/release/test-publish.sh

release-preflight release-verify release-prepare-version release-prepared-check \
release-files release-commit-check release-committed-check release-tagged-check release-push-check:
	+@bash scripts/release/release.sh "$(@:release-%=%)"

publish:
	+@bash scripts/release/publish.sh

install-validation-tools:
	@bash scripts/ci/install-ci-tool.sh shellcheck --install-dir "$(CURDIR)/.tools/ci/bin"
	@bash scripts/ci/install-ci-tool.sh actionlint --install-dir "$(CURDIR)/.tools/ci/bin"
	@bash scripts/ci/install-ci-tool.sh gitleaks --install-dir "$(CURDIR)/.tools/ci/bin"

lint:
	"$(SHELLCHECK)" --external-sources scripts/release/*.sh scripts/ci/check-runtime-identities.sh scripts/ci/managed-ic.sh .githooks/pre-commit
	"$(ACTIONLINT)" .github/workflows/ci.yml

check-secrets:
	"$(GITLEAKS)" git --redact=100 --no-banner --no-color --log-level error "$(CURDIR)"
