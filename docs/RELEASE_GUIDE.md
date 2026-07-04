# Release Guide

This guide explains how to release new versions of netglance.

## Quick Reference

```bash
# Create a release
make release VERSION=0.3.0

# After PR merges, tag the release
make tag-release

# Or manually:
# git tag -a v0.3.0 -m "Release v0.3.0" && git push origin v0.3.0
```

---

## Prerequisites

- `gh` CLI installed and authenticated (`gh auth login`)
- `gitleaks` installed (`brew install gitleaks` on macOS)
- Pre-commit hook installed (`make install-hooks`)
- On `main` branch with clean working directory
- Local `main` branch up to date with `origin/main`

---

## Complete Example: Bugfix Release

### Scenario
You've fixed a bug and want to release version `0.2.1`.

### Step 1: Merge Feature/Bugfix PR

```bash
# Create bugfix branch
git checkout main
git pull
git checkout -b fix/connection-timeout

# Make your changes
vim src/probes/tcp.rs

# Commit changes
git add src/probes/tcp.rs
git commit -m "Fix TCP connection timeout handling"

# Push and create PR
git push -u origin fix/connection-timeout
gh pr create --title "Fix TCP connection timeout" --body "Fixes timeout handling in TCP probes"

# Review, wait for CI, then merge
gh pr merge --squash
```

**What CI checks:** `make static` (format, lint, dead code, unit tests, security audit) + build

### Step 2: Create Release PR

```bash
# Switch back to main
git checkout main
git pull

# Create release PR
make release VERSION=0.2.1
```

**What happens:**
1. Validates prerequisites (gh CLI, main branch, clean git)
2. Validates version format (`0.2.1` without 'v' prefix)
3. Creates branch `release/v0.2.1`
4. Updates `Cargo.toml` version to `0.2.1`
5. Commits: "Bump version to 0.2.1"
6. Pushes branch to GitHub
7. Creates PR with checklist

**Output:**
```
╔══════════════════════════════════════════════════════════════╗
║              netglance Release Process                      ║
╚══════════════════════════════════════════════════════════════╝

✓ Prerequisites checked
✓ On main branch
✓ Working directory clean and up to date
✓ Version 0.2.1 is valid semver

Creating release PR for v0.2.1...
✓ Updated Cargo.toml to 0.2.1
✓ Pushed branch release/v0.2.1

✓✓✓ Release PR created! ✓✓✓

View PR: https://github.com/nmarks/netglance/pull/123

Next steps:
  1. Review the PR and wait for CI checks to pass
  2. Merge the PR when ready
  3. Tag will be created automatically - monitor with: gh run watch
```

### Step 3: Review and Merge Release PR

**On GitHub:**
1. Review the PR at the URL shown above
2. Wait for CI checks to pass:
   - ✓ Static analysis (`make static`)
   - ✓ Integration tests (runs on release PRs)
   - ✓ Build release binary
   - ✓ Gitleaks secret scanning
3. Merge the PR (squash or merge commit)

### Step 4: Tag and Publish Release

**After merge, tag the release:**
```bash
# Switch to main and pull the merged changes
git checkout main
git pull

# Tag the release (automated)
make tag-release
```

**What `make tag-release` does:**
1. Detects version from `Cargo.toml` → `0.2.1`
2. Confirms with you before proceeding
3. Creates annotated tag `v0.2.1`
4. Pushes tag to GitHub
5. Shows commands to monitor progress

**Alternatively, tag manually:**
```bash
git tag -a v0.2.1 -m "Release v0.2.1"
git push origin v0.2.1
```

**What happens next:**
1. Tag push triggers release workflow
2. Binaries are built for Linux and macOS
3. GitHub release is created with all assets

**Monitor progress:**
```bash
# Watch the workflow
gh run watch

# Check when complete
gh release view v0.2.1
```

### Step 5: Verify Release

**The release workflow builds:**
- Linux x86_64 binary: `netglance-0.2.1-linux-x86_64`
- macOS ARM binary: `netglance-0.2.1-darwin-aarch64`
- SHA256 checksums: `checksums.txt`

**Verify:**
```bash
# View release on GitHub
gh release view v0.2.1 --web

# Or check assets
gh release view v0.2.1

# Output:
# netglance v0.2.1
# Published about 5 minutes ago
#
# ASSETS
#   netglance-0.2.1-linux-x86_64        8.1 MB
#   netglance-0.2.1-darwin-aarch64      7.9 MB
#   checksums.txt                        163 bytes
```

**Test download:**
```bash
# Download Linux binary
wget https://github.com/nmarks/netglance/releases/download/v0.2.1/netglance-0.2.1-linux-x86_64

# Verify checksum
wget https://github.com/nmarks/netglance/releases/download/v0.2.1/checksums.txt
sha256sum -c checksums.txt --ignore-missing

# Test binary
chmod +x netglance-0.2.1-linux-x86_64
./netglance-0.2.1-linux-x86_64 --version
# Output: netglance 0.2.1 (abc1234)
```

---

## Version Numbering

Follow [Semantic Versioning](https://semver.org/):

- **PATCH** (0.0.X): Bug fixes, backwards compatible
  - Example: `0.2.0` → `0.2.1` (fixed timeout bug)
  
- **MINOR** (0.X.0): New features, backwards compatible
  - Example: `0.2.1` → `0.3.0` (added HTTP probe support)
  
- **MAJOR** (X.0.0): Breaking changes
  - Example: `0.3.0` → `1.0.0` (changed config file format)

---

## Troubleshooting

### "Error: gh not authenticated"
```bash
gh auth login
```

### "Error: Must be on main branch"
```bash
git checkout main
git pull
```

### "Error: Working directory has uncommitted changes"
```bash
# Commit your changes
git add -A
git commit -m "Your changes"

# Or stash them
git stash
```

### "Error: Local branch is not up to date"
```bash
git pull
```

### CI checks fail on release PR
```bash
# Run checks locally
make static

# If tests fail
make test

# If formatting fails
make fmt

# If linting fails
make clippy
```

### Gitleaks detects false positive
Edit `.gitleaks.toml` and add to allowlist:
```toml
[allowlist]
regexes = [
  '''your-false-positive-pattern''',
]
```

### GitHub Actions workflow fails
```bash
# View logs
gh run list --workflow=release.yml
gh run view <RUN_ID> --log

# Common issues:
# - Rust compilation error: Check with cargo build --release locally
# - Tag already exists: Delete tag and retry
# - Permissions issue: Check repo settings
```

---

## Why Manual Tagging?

**GitHub Actions security feature:** When a workflow uses `GITHUB_TOKEN` to push tags, it intentionally does NOT trigger other workflows. This prevents infinite workflow loops.

**The problem with automatic tagging:**
1. Release PR merges → triggers auto-tag workflow
2. Auto-tag creates tag using `GITHUB_TOKEN`
3. Tag push **doesn't trigger** release workflow ❌
4. No binaries are built

**The solution:** Manual tagging from your local machine DOES trigger the release workflow because it's not coming from GitHub Actions.

---

## Appendix A: How It Works

### Release Workflow Architecture

```
Developer               GitHub                  CI/CD
    |                      |                      |
    | make release         |                      |
    |--------------------->|                      |
    |                      |                      |
    | Creates PR           |                      |
    |<---------------------|                      |
    |                      |                      |
    | Merge PR             |                      |
    |--------------------->|                      |
    |                      |                      |
    | git checkout main    |                      |
    | git pull             |                      |
    |                      |                      |
    | git tag -a v0.2.1    |                      |
    | git push origin v*   |                      |
    |--------------------->|                      |
    |                      |                      |
    |                      | Trigger: release     |
    |                      |--------------------->|
    |                      |                      |
    |                      | Builds binaries      |
    |                      | Creates release      |
    |                      |<---------------------|
    |                      |                      |
    | View release         |                      |
    |<---------------------|                      |
```

### Components

**1. `make release VERSION=X.Y.Z`** (Makefile)
- Validates prerequisites and version format
- Creates release branch `release/vX.Y.Z`
- Updates `Cargo.toml` version
- Commits and pushes
- Creates GitHub PR

**2. CI Workflow** (`.github/workflows/ci.yml`)
- Runs on all PRs
- `static-analysis` job: `make static` (format, lint, tests, security)
- `integration-tests` job: Only on release PRs
- `build` job: `make build-release`

**3. Release Workflow** (`.github/workflows/release.yml`)
- Triggers on tag push matching `v*`
- Builds binaries for Linux x86_64 and macOS ARM
- Generates SHA256 checksums
- Creates GitHub release with all assets

**4. Gitleaks Workflow** (`.github/workflows/gitleaks.yml`)
- Runs on all PRs
- Scans for secrets in commit history
- Fails PR if secrets detected

### File Changes During Release

**Before release:**
```toml
# Cargo.toml
[package]
version = "0.2.0"
```

**After `make release VERSION=0.2.1`:**
```toml
# Cargo.toml (on release/v0.2.1 branch)
[package]
version = "0.2.1"
```

**After merge:**
- Main branch has version `0.2.1`
- Tag `v0.2.1` created automatically
- GitHub release created with binaries

### Version Embedding

Each binary embeds:
- **Version** from `Cargo.toml` (e.g., `0.2.1`)
- **Git hash** from build commit (e.g., `abc1234`)

**Build script** (`build.rs`):
```rust
// Captures git hash at build time
let git_hash = Command::new("git")
    .args(&["rev-parse", "--short", "HEAD"])
    .output()?;
```

**Displayed as:**
```bash
$ netglance --version
netglance 0.2.1 (abc1234)

# In UI footer:
[q]uit [s]ort [f]filter [r]efresh | v0.2.1-abc1234
```

---

## Appendix B: CI/CD Details

### Static Analysis (`make static`)

Runs on every PR:
```bash
make fmt-check     # Code formatting
make clippy-basic  # Linting (warnings as errors)
make dead-code     # Unused code detection
make unit-test     # Unit tests
make audit         # Security vulnerabilities (cargo-audit)
```

### Integration Tests

Run only on release PRs (branch name `release/v*`):
```bash
make integration-test
```

**Why conditional?**
- Integration tests are slower
- Full coverage on release PRs where it matters
- Faster CI feedback on regular PRs

### Security Scanning

**Pre-commit hook:**
- Runs `gitleaks protect --staged` before each commit
- Scans only staged changes
- Fast, immediate feedback
- Developer can bypass with `--no-verify` (not recommended)

**GitHub Actions:**
- Runs `gitleaks detect` on entire repository
- Scans all commits in PR
- Cannot be bypassed
- Catches anything that bypassed pre-commit hook

**What gitleaks detects:**
- AWS credentials (access keys, secret keys, session tokens)
- GitHub tokens (PAT, fine-grained tokens, OAuth)
- Private keys (RSA, DSA, EC, OpenSSH, PGP)
- API keys (generic pattern + service-specific)
- Database connection strings
- Passwords in URLs
- JWT tokens
- High-entropy strings (potential secrets)

---

## Appendix C: Build System

### Build Profiles

**Debug build:**
```bash
make build
# - Fast compilation
# - Includes debug symbols
# - No optimizations
# - Dirty git state allowed
```

**Release build:**
```bash
make build-release
# - Optimized compilation
# - Stripped debug symbols
# - Link-time optimization
# - Clean git state required
```

**Optimized build:**
```bash
make build-optimized
# - Release build + native CPU optimizations
# - Smallest binary size
# - May not be portable
```

### Platform Targets

**GitHub Actions builds:**
- Linux x86_64: `x86_64-unknown-linux-gnu` (ubuntu-latest)
- macOS ARM: `aarch64-apple-darwin` (macos-latest with M-series)

**Not currently built (future):**
- Windows x86_64: `x86_64-pc-windows-msvc`
- Linux ARM: `aarch64-unknown-linux-gnu`
- macOS Intel: `x86_64-apple-darwin`

### Git State Requirements

**Release builds require clean git:**
```bash
# This fails if git is dirty:
cargo build --release

# Error: BUILD FAILED: UNCLEAN GIT STATE
# Release builds require a clean git working directory.
```

**Override (not recommended):**
```bash
NETGLANCE_ALLOW_DIRTY=1 cargo build --release
```

**Debug builds allow dirty git:**
```bash
# This works with uncommitted changes:
cargo build

# Binary version shows dirty state:
./target/debug/netglance --version
# netglance 0.2.1 (abc1234-dirty)
```

---

## Appendix D: Makefile Targets

### Development
| Target | Description |
|--------|-------------|
| `make build` | Build debug binary (fast) |
| `make build-release` | Build optimized release binary |
| `make build-optimized` | Build with native CPU optimizations |
| `make run` | Run debug binary |
| `make check` | Quick compilation check |
| `make clean` | Remove build artifacts |

### Testing
| Target | Description |
|--------|-------------|
| `make test` | Run all tests (unit + integration) |
| `make unit-test` | Run unit tests only |
| `make integration-test` | Run integration tests only |

### Code Quality
| Target | Description |
|--------|-------------|
| `make fmt` | Format code with rustfmt |
| `make fmt-check` | Check formatting (CI) |
| `make clippy` | Run linter with strict checks |
| `make clippy-basic` | Run basic linter (CI) |
| `make dead-code` | Check for unused code |
| `make audit` | Security vulnerability scan |
| `make static` | **All CI checks** (format, lint, test, audit) |

### Security
| Target | Description |
|--------|-------------|
| `make install-hooks` | Install gitleaks pre-commit hook |
| `make uninstall-hooks` | Remove pre-commit hook |
| `make test-gitleaks` | Test gitleaks on entire repo |

### Release
| Target | Description |
|--------|-------------|
| `make release VERSION=X.Y.Z` | Create release PR |
| `make tag-release` | Tag and push release (after PR merges) |
| `make ci` | Run full CI suite locally |

---

## Appendix E: GitHub Actions Workflows

### `ci.yml` - Continuous Integration
**Trigger:** All PRs and pushes to main

**Jobs:**
1. **static-analysis**
   - Runs `make static`
   - Installs `cargo-audit`
   - Caches Rust dependencies

2. **integration-tests** (conditional)
   - Only runs on `release/v*` branches
   - Runs `make integration-test`

3. **build**
   - Runs `make build-release`
   - Validates compilation succeeds

4. **all-checks**
   - Waits for required jobs
   - Reports final status

### `release.yml` - Release Build
**Trigger:** Tag push matching `v*`

**Jobs:**
1. **build** (matrix: Linux, macOS)
   - Checkout at tagged commit
   - Install Rust toolchain
   - Build release binary
   - Rename: `netglance-X.Y.Z-PLATFORM`
   - Generate SHA256 checksum
   - Upload artifacts

2. **release**
   - Download all artifacts
   - Combine checksums → `checksums.txt`
   - Generate release notes
   - Create GitHub release
   - Upload binaries + checksums

### `gitleaks.yml` - Secret Scanning
**Trigger:** All PRs and pushes to main/master/develop

**Steps:**
1. Checkout with full history
2. Run `gitleaks/gitleaks-action@v2.4.0`
3. Fail if secrets detected

---

## Appendix F: Configuration Files

### `.gitleaks.toml`
Gitleaks configuration for secret detection:
- Extends default rules
- Custom rules for AWS, GitHub, Slack, JWT
- Allowlist for false positives
- Entropy detection (threshold: 4.0)
- Paths to ignore (lock files, test data)

**Key sections:**
```toml
[extend]
useDefault = true

[allowlist]
regexes = [...]
paths = [...]

[[rules]]
id = "aws-access-key"
regex = '''...'''
tags = ["aws", "credentials"]

[entropy]
enabled = true
threshold = 4.0
```

### `.githooks/pre-commit`
Pre-commit hook script:
- Checks if gitleaks installed
- Runs `gitleaks protect --staged`
- Provides helpful error messages
- Allows bypass with `--no-verify`

### `.github/dependabot.yml`
Dependabot configuration:
- Weekly updates (Monday 9 AM ET)
- Cargo dependencies (grouped)
- GitHub Actions
- Auto-labels PRs

**Groups:**
- `async-runtime`: tokio, futures
- `tui-framework`: ratatui, crossterm
- `logging`: tracing, log
- `dev-tools`: clap, anyhow, thiserror

---

## Appendix G: Rollback Procedures

### Undo a Release Tag (Before GitHub Release)

```bash
# Delete local tag
git tag -d v0.2.1

# Delete remote tag
git push origin :refs/tags/v0.2.1
```

### Delete a GitHub Release

```bash
# Delete release and tag
gh release delete v0.2.1 --yes

# Delete tag if still exists
git push origin :refs/tags/v0.2.1
```

### Revert Version Bump

```bash
# On main branch
git revert <commit-hash>
git push origin main
```

### Rollback After Issues Discovered

If a release has critical issues:

1. **Delete the release:**
   ```bash
   gh release delete v0.2.1 --yes
   ```

2. **Create hotfix:**
   ```bash
   git checkout -b hotfix/critical-bug
   # Fix the issue
   git commit -m "Fix critical bug"
   git push -u origin hotfix/critical-bug
   gh pr create --title "Hotfix: Critical bug"
   ```

3. **Release fixed version:**
   ```bash
   # After hotfix merges
   make release VERSION=0.2.2
   ```

---

## Appendix H: Best Practices

### Before Creating Release PR

- [ ] All feature PRs merged to main
- [ ] Tests passing locally (`make ci`)
- [ ] Documentation updated
- [ ] RELEASE_NOTES.md updated (if maintained)
- [ ] No known critical bugs
- [ ] Version number decided (semver)

### During Release PR Review

- [ ] CI checks pass
- [ ] Integration tests pass
- [ ] No gitleaks violations
- [ ] Version number correct in Cargo.toml
- [ ] Git commit message follows convention

### After Release

- [ ] GitHub release created successfully
- [ ] All binaries uploaded
- [ ] Checksums present
- [ ] Download and test binaries
- [ ] Announce release (if applicable)

### Security & Quality Checklist

- [ ] Pre-commit hook installed (`make install-hooks`)
- [ ] Pre-commit hook runs gitleaks + make static automatically
- [ ] Never commit with `--no-verify` unless certain
- [ ] Review gitleaks output carefully
- [ ] Add false positives to `.gitleaks.toml` allowlist
- [ ] Run `make test-gitleaks` periodically
- [ ] Update dependencies regularly (Dependabot PRs)
- [ ] Review `make audit` output
- [ ] All commits pass local static checks before push

---

**Last Updated:** 2026-07-04
