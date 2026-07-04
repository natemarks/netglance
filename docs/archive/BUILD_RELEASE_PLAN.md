# Build & Release Automation Plan

**Goal**: Add Makefile targets for building and releasing netglance with semantic versioning.

---

## Build Targets

### Basic Build Targets

```makefile
# Build debug binary
build-debug:
	cargo build
	
# Build release binary (optimized)
build-release:
	cargo build --release
	
# Build with all optimizations
build-optimized:
	RUSTFLAGS="-C target-cpu=native" cargo build --release
	
# Clean build artifacts
clean:
	cargo clean
	rm -f netglance-*.tar.gz
```

### Platform-Specific Builds

```makefile
# Build for Linux x86_64
build-linux-x64:
	cargo build --release --target x86_64-unknown-linux-gnu
	
# Build for Linux ARM64
build-linux-arm64:
	cargo build --release --target aarch64-unknown-linux-gnu
	
# Build for macOS (Intel)
build-macos-x64:
	cargo build --release --target x86_64-apple-darwin
	
# Build for macOS (Apple Silicon)
build-macos-arm64:
	cargo build --release --target aarch64-apple-darwin
```

### Binary Size Optimization

```makefile
# Strip debug symbols and optimize size
build-minimal:
	cargo build --release
	strip target/release/netglance
	
# Use UPX compression (optional)
build-compressed:
	cargo build --release
	strip target/release/netglance
	upx --best --lzma target/release/netglance
```

---

## Release Targets

### GitHub Release with gh CLI

**Requirements**:
- `gh` CLI installed and authenticated
- Git repository with remote
- Semantic version as input (e.g., v0.2.0)

**Process**:
1. Validate semantic version format
2. Update version in Cargo.toml
3. Run tests to ensure everything passes
4. Build release binary
5. Create git tag
6. Create GitHub release
7. Upload binary artifact

### Makefile Target Design

```makefile
# Release to GitHub with semantic version
# Usage: make release VERSION=v0.2.0
release: VERSION ?= $(error VERSION is required, use: make release VERSION=v0.2.0)
release: check-gh-cli validate-version update-version test build-release
	@echo "Creating release $(VERSION)..."
	git add Cargo.toml Cargo.lock
	git commit -m "Release $(VERSION)"
	git tag -a $(VERSION) -m "Release $(VERSION)"
	git push origin main $(VERSION)
	gh release create $(VERSION) \
		--title "netglance $(VERSION)" \
		--notes-file RELEASE_NOTES.md \
		target/release/netglance#netglance-$(VERSION)-linux-x64
	@echo "✓ Release $(VERSION) published!"
```

### Helper Targets

```makefile
# Check if gh CLI is installed
check-gh-cli:
	@which gh > /dev/null || (echo "Error: gh CLI not found. Install from https://cli.github.com" && exit 1)
	@gh auth status || (echo "Error: gh CLI not authenticated. Run: gh auth login" && exit 1)

# Validate semantic version format
validate-version:
	@echo "$(VERSION)" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+$$' || \
		(echo "Error: Invalid version format. Use: vMAJOR.MINOR.PATCH (e.g., v0.2.0)" && exit 1)
	@echo "✓ Version $(VERSION) is valid"

# Update version in Cargo.toml
update-version:
	@VERSION_NUM=$$(echo $(VERSION) | sed 's/^v//'); \
	sed -i "s/^version = \".*\"/version = \"$$VERSION_NUM\"/" Cargo.toml
	@echo "✓ Updated Cargo.toml to $(VERSION)"

# Generate release notes
generate-release-notes:
	@echo "# Release $(VERSION)" > RELEASE_NOTES_TEMP.md
	@echo "" >> RELEASE_NOTES_TEMP.md
	@echo "## What's New" >> RELEASE_NOTES_TEMP.md
	@echo "" >> RELEASE_NOTES_TEMP.md
	@git log --pretty=format:"- %s" $(shell git describe --tags --abbrev=0)..HEAD >> RELEASE_NOTES_TEMP.md
	@echo "" >> RELEASE_NOTES_TEMP.md
	@echo "" >> RELEASE_NOTES_TEMP.md
	@echo "## Installation" >> RELEASE_NOTES_TEMP.md
	@echo "" >> RELEASE_NOTES_TEMP.md
	@echo "\`\`\`bash" >> RELEASE_NOTES_TEMP.md
	@echo "# Download and extract" >> RELEASE_NOTES_TEMP.md
	@echo "wget https://github.com/nmarks/netglance/releases/download/$(VERSION)/netglance-$(VERSION)-linux-x64" >> RELEASE_NOTES_TEMP.md
	@echo "chmod +x netglance-$(VERSION)-linux-x64" >> RELEASE_NOTES_TEMP.md
	@echo "mv netglance-$(VERSION)-linux-x64 netglance" >> RELEASE_NOTES_TEMP.md
	@echo "\`\`\`" >> RELEASE_NOTES_TEMP.md
```

---

## Multi-Platform Release

### Build for Multiple Platforms

```makefile
# Build binaries for all supported platforms
build-all-platforms:
	@echo "Building for all platforms..."
	cargo build --release --target x86_64-unknown-linux-gnu
	cargo build --release --target aarch64-unknown-linux-gnu
	cargo build --release --target x86_64-apple-darwin
	cargo build --release --target aarch64-apple-darwin
	@echo "✓ Built all platform binaries"

# Package binaries with platform suffix
package-binaries:
	@mkdir -p dist
	cp target/x86_64-unknown-linux-gnu/release/netglance dist/netglance-$(VERSION)-linux-x64
	cp target/aarch64-unknown-linux-gnu/release/netglance dist/netglance-$(VERSION)-linux-arm64
	cp target/x86_64-apple-darwin/release/netglance dist/netglance-$(VERSION)-macos-x64
	cp target/aarch64-apple-darwin/release/netglance dist/netglance-$(VERSION)-macos-arm64
	@echo "✓ Packaged binaries to dist/"

# Release with all platform binaries
release-all-platforms: VERSION ?= $(error VERSION required)
release-all-platforms: build-all-platforms package-binaries
	gh release create $(VERSION) \
		--title "netglance $(VERSION)" \
		--notes-file RELEASE_NOTES.md \
		dist/netglance-$(VERSION)-linux-x64 \
		dist/netglance-$(VERSION)-linux-arm64 \
		dist/netglance-$(VERSION)-macos-x64 \
		dist/netglance-$(VERSION)-macos-arm64
```

---

## Pre-Release Validation

### Checklist Target

```makefile
# Pre-release checklist
pre-release-check:
	@echo "Running pre-release checks..."
	@echo "→ Checking git status..."
	@git diff-index --quiet HEAD || (echo "✗ Uncommitted changes detected" && exit 1)
	@echo "✓ Working directory clean"
	@echo "→ Running tests..."
	@make test > /dev/null 2>&1 || (echo "✗ Tests failed" && exit 1)
	@echo "✓ All tests passing"
	@echo "→ Running CI checks..."
	@make ci > /dev/null 2>&1 || (echo "✗ CI checks failed" && exit 1)
	@echo "✓ CI checks passing"
	@echo "→ Checking version consistency..."
	@grep -q "version = \"$(VERSION_NUM)\"" Cargo.toml || (echo "✗ Version mismatch" && exit 1)
	@echo "✓ Pre-release checks complete!"
```

---

## GitHub Actions Integration (Optional)

### Automated Release Workflow

**`.github/workflows/release.yml`**:
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  build-and-release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Build release
        run: cargo build --release
        
      - name: Create GitHub Release
        uses: softprops/action-gh-release@v1
        with:
          files: target/release/netglance
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

## Implementation Checklist

### Phase 1: Basic Build Targets
- [ ] Add `build-debug` target
- [ ] Add `build-release` target
- [ ] Add `clean` target
- [ ] Test build targets work

### Phase 2: Release Infrastructure
- [ ] Add `check-gh-cli` target
- [ ] Add `validate-version` target
- [ ] Add `update-version` target
- [ ] Test version validation

### Phase 3: Release Target
- [ ] Add `release` target
- [ ] Test with dry-run
- [ ] Document usage in README
- [ ] Test actual release

### Phase 4: Multi-Platform (Optional)
- [ ] Add cross-compilation targets
- [ ] Add platform-specific build targets
- [ ] Add `release-all-platforms` target
- [ ] Set up cross-compilation toolchains

---

## Usage Examples

### Simple Release
```bash
# Make a new release
make release VERSION=v0.2.0
```

### Manual Release Steps
```bash
# 1. Build release binary
make build-release

# 2. Run tests
make test

# 3. Create release
make release VERSION=v0.2.0
```

### Multi-Platform Release
```bash
# Build for all platforms and release
make release-all-platforms VERSION=v0.2.0
```

### Pre-Release Validation
```bash
# Validate before releasing
make pre-release-check VERSION=v0.2.0
```

---

## Safety Features

### Built-in Validations

1. **Version Format**: Must match `vMAJOR.MINOR.PATCH`
2. **gh CLI Check**: Ensures GitHub CLI is installed and authenticated
3. **Git Status**: Warns if there are uncommitted changes
4. **Test Execution**: Runs full test suite before release
5. **Tag Uniqueness**: Git will fail if tag already exists

### Rollback Strategy

If release fails:
```bash
# Delete local tag
git tag -d v0.2.0

# Delete remote tag (if pushed)
git push origin :refs/tags/v0.2.0

# Delete GitHub release
gh release delete v0.2.0
```

---

## Future Enhancements

- [ ] Automated changelog generation
- [ ] GPG signing of releases
- [ ] Checksums for binaries
- [ ] Docker image releases
- [ ] Homebrew formula updates
- [ ] Cargo registry publishing
- [ ] Release announcement automation

---

_Last updated: 2026-07-03_
