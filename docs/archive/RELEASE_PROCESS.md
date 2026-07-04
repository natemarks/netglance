# Release Process

This document describes how to create a new release of netglance.

## Prerequisites

- [ ] `gh` CLI installed and authenticated (`gh auth login`)
- [ ] On `main` branch with clean working directory
- [ ] All changes merged via PRs
- [ ] Local `main` branch up to date with `origin/main`
- [ ] `gitleaks` installed for pre-commit hook (`brew install gitleaks` on macOS)

## Release Workflow

netglance uses a **Release PR workflow with automatic tagging**:

```
Feature PRs → Main → Release PR → Merge → Auto-Tag → GitHub Actions → Release
```

### Step 1: Create Release PR

```bash
# Ensure you're on main and up to date
git checkout main
git pull

# Start release process with version
make release VERSION=0.3.0
```

**What happens:**
1. Validates prerequisites (gh CLI, main branch, clean git)
2. Validates version format (format: `0.2.1` without 'v' prefix)
3. Creates branch `release/v0.3.0`
4. Updates `Cargo.toml` version to `0.3.0`
5. Commits: "Bump version to 0.3.0"
6. Pushes branch to GitHub
7. Creates PR with checklist

**Example:**
```bash
$ make release VERSION=0.3.0
╔══════════════════════════════════════════════════════════════╗
║              netglance Release Process                      ║
╚══════════════════════════════════════════════════════════════╝

✓ Prerequisites checked
✓ On main branch
✓ Working directory clean and up to date
✓ Version 0.3.0 is valid semver

Creating release PR for v0.3.0...
✓ Updated Cargo.toml to 0.3.0
✓ Pushed branch release/v0.3.0

✓✓✓ Release PR created! ✓✓✓

View PR: https://github.com/nmarks/netglance/pull/123

Next steps:
  1. Review the PR and wait for CI checks to pass
  2. Merge the PR when ready
  3. Tag will be created automatically - monitor with: gh run watch
```

### Step 2: Review and Merge PR

1. **Review the PR** on GitHub
2. **Wait for CI checks** to pass:
   - Static analysis (`make static`):
     - Formatting (`make fmt-check`)
     - Linting (`make clippy-basic`)
     - Dead code check
     - Unit tests
     - Security audit (`cargo audit`)
   - Integration tests (runs only on release PRs)
   - Build release binary
   - Gitleaks secret scanning
3. **Merge the PR** (squash or merge commit)

### Step 3: Automatic Tagging

After the PR is merged to main, **GitHub Actions automatically**:

1. Detects the "Bump version to X.Y.Z" commit
2. Extracts version from `Cargo.toml`
3. Creates annotated tag `vX.Y.Z`
4. Pushes tag to GitHub
5. Tag push triggers the release build workflow

**No manual tagging step required!** Monitor progress with:
```bash
gh run watch
```

### Step 4: GitHub Actions Builds Release

Once the tag is pushed, GitHub Actions automatically:

1. **Builds binaries** on native platforms:
   - Linux x86_64 (`ubuntu-latest`)
   - macOS ARM (`macos-latest` with M-series runner)

2. **Names binaries** with version and platform:
   - `netglance-0.3.0-linux-x86_64`
   - `netglance-0.3.0-darwin-aarch64`

3. **Generates SHA256 checksums** for each binary:
   - `netglance-0.3.0-linux-x86_64.sha256`
   - `netglance-0.3.0-darwin-aarch64.sha256`

4. **Creates single checksums file**:
   - `checksums.txt` containing all checksums

5. **Creates GitHub release**:
   - Title: "netglance v0.3.0"
   - Includes installation instructions
   - Attaches all binaries and checksums

### Step 5: Monitor and Verify

**Monitor the build:**
```bash
gh run watch
```

**View the release:**
```bash
# Open in browser
gh release view v0.3.0 --web

# List assets
gh release view v0.3.0
```

**Verify downloads:**
```bash
# Download and verify checksums
wget https://github.com/nmarks/netglance/releases/download/v0.3.0/netglance-0.3.0-linux-x86_64
wget https://github.com/nmarks/netglance/releases/download/v0.3.0/checksums.txt

sha256sum -c checksums.txt --ignore-missing
```

## Version Display

Each release binary embeds:
- **Version number** from `Cargo.toml`
- **Git commit hash** (7 chars) from the tagged commit

**Displayed in:**
1. **Footer of UI**: `[q]uit [s]ort [f]filter [r]efresh | v0.3.0-abc1234`
2. **--version flag**: `netglance 0.3.0 (abc1234)`

**Git state detection:**
- Clean builds: `v0.3.0-abc1234`
- Dirty builds: `v0.3.0-abc1234-dirty` (release builds fail by default)

## Git State Requirements

### Release Builds (Production)

**Requirement:** Git working directory MUST be clean.

```bash
# Release build with dirty git fails:
$ cargo build --release
error: BUILD FAILED: UNCLEAN GIT STATE

Release builds require a clean git working directory.
```

**Override (not recommended):**
```bash
NETGLANCE_ALLOW_DIRTY=1 cargo build --release
```

### Debug Builds (Development)

**Allowed:** Uncommitted changes are fine.

```bash
# Debug build with dirty git works:
$ cargo build
$ ./target/debug/netglance --version
netglance 0.3.0 (abc1234-dirty)
```

## Makefile Targets

### `make release VERSION=X.Y.Z`
Start the release process. Creates release PR with version bump.

**Usage:** `make release VERSION=0.3.0`

**Requirements:**
- On `main` branch
- Clean working directory
- Up to date with `origin/main`
- `gh` CLI authenticated
- Version format: `X.Y.Z` (e.g., `0.3.0`)

**After merge:** Tag is automatically created by GitHub Actions

### `make static`
Run comprehensive static analysis checks (used in CI).

**Includes:**
- `make fmt-check` - Code formatting
- `make clippy-basic` - Linting
- `make dead-code` - Unused code detection
- `make unit-test` - Unit tests
- `make audit` - Security audit

### `make install-hooks`
Install git pre-commit hook for gitleaks secret scanning.

**Requirements:** `gitleaks` must be installed (`brew install gitleaks` on macOS)

### `make test-gitleaks`
Test gitleaks configuration on entire repository.

## GitHub Actions Workflows

### Auto-Tag Workflow (`.github/workflows/auto-tag.yml`)

**Trigger:** Push to `main` branch that modifies `Cargo.toml`

**Conditions:** Only runs if commit message contains "Bump version to"

**Steps:**
1. Extract version from `Cargo.toml`
2. Check if tag already exists
3. Create annotated tag `vX.Y.Z`
4. Push tag to GitHub
5. Tag push triggers release workflow

### Release Workflow (`.github/workflows/release.yml`)

**Trigger:** Push of tag matching `v*` pattern

**Jobs:**

1. **build** (matrix: Linux x86_64, macOS ARM)
   - Checkout code at tagged commit
   - Install Rust toolchain
   - Build release binary with embedded git hash
   - Rename binary: `netglance-X.Y.Z-PLATFORM`
   - Generate SHA256 checksum
   - Upload artifacts

2. **release** (depends on build)
   - Download all artifacts
   - Combine checksums into `checksums.txt`
   - Generate release notes
   - Create GitHub release with `gh` CLI
   - Upload all binaries and checksums

### CI Workflow (`.github/workflows/ci.yml`)

**Trigger:** Pull requests and pushes to `main`

**Jobs:**

1. **static-analysis** - Runs `make static` (comprehensive checks)
2. **integration-tests** - Runs only on release PRs (`release/v*` branches)
3. **build** - Builds release binary
4. **all-checks** - Validates all required checks passed

### Gitleaks Workflow (`.github/workflows/gitleaks.yml`)

**Trigger:** Pull requests and pushes to `main`, `master`, `develop`

**Job:** Scans for secrets in commit history using gitleaks

## Troubleshooting

### "Error: gh not authenticated"
```bash
gh auth login
```

### "Error: Must be on main branch"
```bash
git checkout main
```

### "Error: Working directory has uncommitted changes"
```bash
# Commit changes
git add -A
git commit -m "Your changes"

# Or stash them
git stash
```

### "Error: Local branch is not up to date"
```bash
git pull
```

### "Build failed: UNCLEAN GIT STATE"
```bash
# Check what's uncommitted
git status

# Commit everything
git add -A
git commit -m "Changes"

# Or allow dirty build (not for release!)
NETGLANCE_ALLOW_DIRTY=1 cargo build --release
```

### CI checks fail on release PR
```bash
# Run checks locally
make ci

# If tests fail
make test

# If formatting fails
make fmt

# If linting fails
make clippy
```

### GitHub Actions build fails

**View logs:**
```bash
gh run list --workflow=release.yml
gh run view <RUN_ID> --log
```

**Common issues:**
- Rust compilation error: Check `cargo build --release` locally
- Missing dependencies: Ensure `Cargo.toml` is correct
- Cross-compilation issue: Test with target explicitly

## Security: Gitleaks Pre-commit Hook

### Installing the Hook

```bash
# Install gitleaks first
brew install gitleaks  # macOS
# or see: https://github.com/gitleaks/gitleaks#installing

# Install the pre-commit hook
make install-hooks
```

### What It Does

The pre-commit hook runs `gitleaks protect` on staged changes to detect:
- AWS credentials (access keys, secret keys, session tokens)
- GitHub tokens (PAT, fine-grained tokens)
- Private keys (RSA, DSA, EC, OpenSSH)
- API keys and tokens
- High-entropy strings (potential secrets)
- Database connection strings
- Passwords in URLs

### Configuration

Gitleaks is configured in `.gitleaks.toml`:
- **Default rules enabled** for common secrets
- **Custom rules** for AWS, GitHub, Slack, JWT tokens
- **Allowlist** for false positives (test data, documentation)
- **Entropy detection** for random strings (threshold: 4.0)

### Testing

```bash
# Test gitleaks on entire repository
make test-gitleaks

# Test pre-commit hook
echo "AKIA1234567890123456" > test.txt
git add test.txt
git commit -m "test"  # Will fail with secret detected
```

### Bypassing (Not Recommended)

```bash
# Skip the hook (only if you're certain it's a false positive)
git commit --no-verify
```

## Example: Full Release Flow

```bash
# 1. Start release
$ make release VERSION=0.2.1
✓✓✓ Release PR created! ✓✓✓

# 2. Review PR, wait for CI, then merge on GitHub
#    - CI runs: make static + integration tests
#    - Gitleaks scans for secrets
#    - Build succeeds

# 3. After merge, tag is created automatically
#    Monitor the auto-tagging:
$ gh run watch

# 4. Monitor release build
$ gh run watch

# 5. Verify release
$ gh release view v0.2.1
netglance v0.2.1
Published about 2 minutes ago

ASSETS
  netglance-0.2.1-linux-x86_64        8.1 MB
  netglance-0.2.1-darwin-aarch64      7.9 MB
  checksums.txt                        163 bytes

# 6. Test download
$ wget https://github.com/nmarks/netglance/releases/download/v0.2.1/netglance-0.2.1-linux-x86_64
$ chmod +x netglance-0.2.1-linux-x86_64
$ ./netglance-0.2.1-linux-x86_64 --version
netglance 0.2.1 (abc1234)
```

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **MAJOR** (X.0.0): Breaking changes
- **MINOR** (0.X.0): New features, backwards compatible
- **PATCH** (0.0.X): Bug fixes, backwards compatible

**Examples:**
- `0.2.1` → `0.2.2`: Bug fix
- `0.2.1` → `0.3.0`: New feature (HTTP probes)
- `0.2.1` → `1.0.0`: Stable release, breaking changes

## Checklist: Before Release

- [ ] All features merged to main
- [ ] Tests passing (`make ci`)
- [ ] Documentation updated
- [ ] RELEASE_NOTES.md updated (if desired)
- [ ] No known critical bugs
- [ ] Version number decided

## Checklist: After Release

- [ ] GitHub release created successfully
- [ ] All binaries uploaded (Linux, macOS)
- [ ] Checksums present
- [ ] Download and test each binary
- [ ] Announce release (if applicable)

---

**Last Updated:** 2026-07-04  
**Version:** 0.2.0
