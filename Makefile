.PHONY: help all build build-debug build-release build-optimized run check clean clean-all
.PHONY: test unit-test integration-test fmt fmt-check clippy clippy-basic doc static static-full ci pre-commit
.PHONY: dead-code unused-deps audit deny bloat
.PHONY: install-hooks uninstall-hooks test-gitleaks test-dependabot-pr
.PHONY: release tag-release _check-prerequisites _check-main-branch _check-git-clean _validate-semver _create-release-pr _extract-version

.DEFAULT_GOAL := help

PROJECT_NAME := netglance
CARGO := cargo

##@ General

help: ## Display this help message
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make <target>\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  %-20s %s\n", $$1, $$2 } /^##@/ { printf "\n%s\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

all: fmt clippy test build-release ## Run all checks and build release

##@ Development

build: build-debug ## Build in debug mode (default target)

build-debug: ## Build debug binary
	@echo "Building debug binary..."
	@$(CARGO) build
	@echo "✓ Debug binary: target/debug/$(PROJECT_NAME)"

build-release: ## Build optimized release binary
	@echo "Building release binary..."
	@$(CARGO) build --release
	@echo "✓ Release binary: target/release/$(PROJECT_NAME)"

build-optimized: ## Build with maximum optimizations
	@echo "Building optimized binary (native CPU + stripped)..."
	@RUSTFLAGS="-C target-cpu=native" $(CARGO) build --release
	@strip target/release/$(PROJECT_NAME)
	@echo "✓ Optimized binary: target/release/$(PROJECT_NAME)"

run: ## Run the application
	@$(CARGO) run

check: ## Quick compilation check
	@$(CARGO) check

clean: ## Remove cargo build artifacts
	@$(CARGO) clean

clean-all: clean ## Remove all build artifacts including dist/
	@rm -rf dist/
	@rm -f $(PROJECT_NAME)-*.tar.gz
	@rm -f RELEASE_NOTES_TEMP.md
	@echo "✓ Cleaned all build artifacts"

##@ Testing

test: unit-test integration-test ## Run all tests

unit-test: ## Run unit tests (library and binary)
	@$(CARGO) test --lib --bins

integration-test: ## Run integration tests
	@$(CARGO) test --test '*'

##@ Code Quality

fmt: ## Format code with rustfmt
	@$(CARGO) fmt

fmt-check: ## Check code formatting
	@$(CARGO) fmt -- --check

clippy: ## Run Clippy linter with strict checks
	@echo "Running Clippy with strict warnings..."
	@$(CARGO) clippy --all-targets --all-features -- \
		-D warnings \
		-D clippy::all \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::module_name_repetitions \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc
	@echo "✓ Clippy passed"

clippy-basic: ## Run basic Clippy checks (used in static target)
	@$(CARGO) clippy --lib --bins --tests -- -D warnings

dead-code: ## Check for unused/dead code
	@echo "Checking for dead code..."
	@RUSTFLAGS="-D dead_code" $(CARGO) check --all-targets 2>&1 | \
		grep -E "(warning|error)" | grep -v "Compiling" || echo "✓ No dead code detected"

unused-deps: ## Check for unused dependencies (requires cargo-udeps)
	@echo "Checking for unused dependencies..."
	@if command -v cargo-udeps > /dev/null 2>&1; then \
		$(CARGO) +nightly udeps --all-targets; \
	else \
		echo "⚠ cargo-udeps not installed. Install with:"; \
		echo "  cargo install cargo-udeps --locked"; \
		echo "  rustup install nightly"; \
	fi

audit: ## Check for security vulnerabilities (requires cargo-audit)
	@echo "Auditing dependencies for security vulnerabilities..."
	@if command -v cargo-audit > /dev/null 2>&1; then \
		$(CARGO) audit; \
	else \
		echo "⚠ cargo-audit not installed (optional). Install with:"; \
		echo "  cargo install cargo-audit --locked"; \
		echo "✓ Skipping audit check"; \
	fi

deny: ## Check dependencies against deny.toml policy (requires cargo-deny)
	@echo "Checking dependency policies..."
	@if command -v cargo-deny > /dev/null 2>&1; then \
		$(CARGO) deny check; \
	else \
		echo "⚠ cargo-deny not installed. Install with:"; \
		echo "  cargo install cargo-deny --locked"; \
	fi

bloat: ## Analyze binary size (requires cargo-bloat)
	@echo "Analyzing binary bloat..."
	@if command -v cargo-bloat > /dev/null 2>&1; then \
		$(CARGO) bloat --release -n 20; \
	else \
		echo "⚠ cargo-bloat not installed. Install with:"; \
		echo "  cargo install cargo-bloat --locked"; \
	fi

doc: ## Generate and open documentation
	@$(CARGO) doc --no-deps --open

static: fmt-check clippy-basic dead-code unit-test audit ## Run essential static analysis (format, lint, dead code, tests, security)
	@echo ""
	@echo "✓✓✓ All static checks passed! ✓✓✓"

static-full: fmt-check clippy dead-code unit-test audit ## Run comprehensive static analysis
	@echo ""
	@echo "✓✓✓ Full static analysis complete! ✓✓✓"
	@echo ""
	@echo "Optional additional checks:"
	@echo "  make unused-deps  - Find unused dependencies"
	@echo "  make deny         - Check dependency policies"
	@echo "  make bloat        - Analyze binary size"

##@ CI/CD

ci: static integration-test ## Run all CI checks (static + integration tests)
	@echo "✓ All CI checks passed"

pre-commit: static ## Run pre-commit checks (same as static)

install-hooks: ## Install git pre-commit hook (gitleaks)
	@if ! command -v gitleaks > /dev/null 2>&1; then \
		echo "⚠ Warning: gitleaks not installed"; \
		echo ""; \
		echo "Install gitleaks:"; \
		echo "  macOS:  brew install gitleaks"; \
		echo "  Linux:  https://github.com/gitleaks/gitleaks#installing"; \
		echo ""; \
		echo "The hook will be installed but will fail until gitleaks is available."; \
		echo ""; \
	fi
	@mkdir -p .git/hooks
	@cp .githooks/pre-commit .git/hooks/pre-commit
	@chmod +x .git/hooks/pre-commit
	@echo "✓ Git pre-commit hook installed (gitleaks)"
	@echo ""
	@if command -v gitleaks > /dev/null 2>&1; then \
		echo "✓ gitleaks is installed"; \
	else \
		echo "⚠ Install gitleaks to enable secret scanning"; \
	fi

uninstall-hooks: ## Remove git pre-commit hook
	@rm -f .git/hooks/pre-commit
	@echo "✓ Git pre-commit hook removed"

test-gitleaks: ## Test gitleaks on current repository
	@if ! command -v gitleaks > /dev/null 2>&1; then \
		echo "✗ Error: gitleaks not installed"; \
		echo "Install: brew install gitleaks (macOS) or see https://github.com/gitleaks/gitleaks"; \
		exit 1; \
	fi
	@echo "Running gitleaks on entire repository..."
	@gitleaks detect --verbose

test-dependabot-pr: clean ## Test Dependabot PR (clean + all checks)
	@echo "=== Testing Dependabot PR ==="
	@echo ""
	@echo "→ Building project..."
	@$(CARGO) build
	@echo ""
	@echo "→ Running CI checks..."
	@$(MAKE) ci
	@echo ""
	@echo "✓ All checks passed!"
	@echo ""
	@echo "If all looks good, merge the PR:"
	@echo "  gh pr merge <PR#> --squash"

##@ Release

release: ## Start release process (Usage: make release VERSION=0.2.1)
ifndef VERSION
	@echo "╔══════════════════════════════════════════════════════════════╗"
	@echo "║              netglance Release Process                      ║"
	@echo "╚══════════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "Usage: make release VERSION=X.Y.Z"
	@echo ""
	@echo "Example: make release VERSION=0.2.1"
	@echo ""
	@exit 1
endif
	@echo "╔══════════════════════════════════════════════════════════════╗"
	@echo "║              netglance Release Process                      ║"
	@echo "╚══════════════════════════════════════════════════════════════╝"
	@echo ""
	@$(MAKE) _check-prerequisites
	@$(MAKE) _check-main-branch
	@$(MAKE) _check-git-clean
	@echo ""
	@$(MAKE) _validate-semver VERSION=$(VERSION)
	@$(MAKE) _create-release-pr VERSION=$(VERSION)

# Internal: Check prerequisites
_check-prerequisites:
	@which gh > /dev/null || (echo "✗ Error: gh CLI not found. Install: https://cli.github.com" && exit 1)
	@gh auth status > /dev/null 2>&1 || (echo "✗ Error: gh not authenticated. Run: gh auth login" && exit 1)
	@echo "✓ Prerequisites checked"

# Internal: Check we're on main branch
_check-main-branch:
	@BRANCH=$$(git branch --show-current); \
	if [ "$$BRANCH" != "main" ]; then \
		echo "✗ Error: Must be on main branch (currently on $$BRANCH)"; \
		exit 1; \
	fi
	@echo "✓ On main branch"

# Internal: Check git working directory is clean
_check-git-clean:
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "✗ Error: Working directory has uncommitted changes"; \
		echo ""; \
		git status --short; \
		echo ""; \
		echo "Commit or stash your changes first."; \
		exit 1; \
	fi
	@git fetch origin
	@LOCAL=$$(git rev-parse @); \
	REMOTE=$$(git rev-parse @{u}); \
	if [ "$$LOCAL" != "$$REMOTE" ]; then \
		echo "✗ Error: Local branch is not up to date with origin/main"; \
		echo "Run: git pull"; \
		exit 1; \
	fi
	@echo "✓ Working directory clean and up to date"

# Internal: Validate semver format (without 'v' prefix)
_validate-semver:
ifndef VERSION
	$(error VERSION is required)
endif
	@echo "$(VERSION)" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$$' || \
		(echo "✗ Error: Invalid version '$(VERSION)'. Format: MAJOR.MINOR.PATCH (e.g., 0.2.1)" && exit 1)
	@echo "✓ Version $(VERSION) is valid semver"

# Internal: Create release PR
_create-release-pr:
	@echo ""
	@echo "Creating release PR for v$(VERSION)..."
	@echo ""
	@git checkout -b release/v$(VERSION)
	@sed -i 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml
	@cargo check > /dev/null 2>&1
	@echo "✓ Updated Cargo.toml to $(VERSION)"
	@echo ""
	@git add Cargo.toml Cargo.lock
	@git commit -m "Bump version to $(VERSION)"
	@git push -u origin release/v$(VERSION)
	@echo "✓ Pushed branch release/v$(VERSION)"
	@echo ""
	@echo "Creating pull request..."
	@printf '%s\n' \
		"## Release v$(VERSION)" \
		"" \
		"This PR prepares the release of version $(VERSION)." \
		"" \
		"### Checklist" \
		"" \
		"- [ ] CI checks pass (make static + integration tests)" \
		"- [ ] Version updated in Cargo.toml" \
		"- [ ] RELEASE_NOTES.md updated (if applicable)" \
		"- [ ] Documentation reviewed" \
		"" \
		"### After Merge" \
		"" \
		"When this PR is merged to main, create and push the tag:" \
		"" \
		"\`\`\`bash" \
		"make tag-release" \
		"\`\`\`" \
		"" \
		"This will:" \
		"1. Create annotated tag v$(VERSION)" \
		"2. Push tag to GitHub" \
		"3. Trigger release workflow" \
		"4. Build binaries (Linux x86_64, macOS ARM)" \
		"5. Create GitHub release with all assets" \
		"" \
		| gh pr create --title "Release v$(VERSION)" --body-file -
	@echo ""
	@echo "✓✓✓ Release PR created! ✓✓✓"
	@echo ""
	@echo "View PR: $$(gh pr view --json url -q .url)"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Review the PR and wait for CI checks to pass"
	@echo "  2. Merge the PR when ready"
	@echo "  3. Run: make tag-release"

tag-release: ## Tag and push release (run after merging release PR)
	@$(MAKE) _check-prerequisites
	@$(MAKE) _check-main-branch
	@$(MAKE) _extract-version
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════════╗"
	@echo "║              Tagging Release                                 ║"
	@echo "╚══════════════════════════════════════════════════════════════╝"
	@echo ""
	@VERSION=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	echo "Detected version: $$VERSION from Cargo.toml"; \
	echo ""; \
	echo "This will:"; \
	echo "  1. Create annotated tag v$$VERSION"; \
	echo "  2. Push tag to GitHub"; \
	echo "  3. Trigger release workflow"; \
	echo ""; \
	read -p "Continue? [y/N] " -n 1 -r REPLY; \
	echo ""; \
	if [ "$$REPLY" != "y" ] && [ "$$REPLY" != "Y" ]; then \
		echo "Aborted."; \
		exit 1; \
	fi; \
	echo ""; \
	echo "Creating and pushing tag v$$VERSION..."; \
	git tag -a "v$$VERSION" -m "Release v$$VERSION"; \
	git push origin "v$$VERSION"; \
	echo ""; \
	echo "✓✓✓ Tag v$$VERSION pushed! ✓✓✓"; \
	echo ""; \
	echo "Monitor release workflow:"; \
	echo "  gh run watch"; \
	echo ""; \
	echo "View release when complete:"; \
	echo "  gh release view v$$VERSION --web"

# Internal: Extract version from Cargo.toml
_extract-version:
	@VERSION=$$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/'); \
	if [ -z "$$VERSION" ]; then \
		echo "✗ Error: Could not extract version from Cargo.toml"; \
		exit 1; \
	fi
