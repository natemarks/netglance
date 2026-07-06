# Dependabot Guide for netglance

This document explains how to handle Dependabot pull requests for the netglance project.

---

## Quick Start

### List and Review PRs

```bash
# Show all open Dependabot PRs
make list-dependabot-prs
```

### Test a Specific PR

```bash
# Checkout and test PR #123
make test-dependabot-pr-full PR=123
```

### Merge a Tested PR

```bash
# Squash merge and cleanup
make merge-dependabot-pr PR=123
```

### Bulk Operations (use with caution)

```bash
# Test and merge all PRs
make merge-all-dependabot-prs
```

---

## Testing Locally

To validate a Dependabot PR on your machine:

```bash
# Option 1: Test PR already checked out
make test-dependabot-pr

# Option 2: Checkout and test in one command
make test-dependabot-pr-full PR=<PR#>
```

This will:
1. Clean build artifacts (`make clean`)
2. Build the project (`cargo build`)
3. Run all CI checks:
   - Format checking (`cargo fmt --check`)
   - Linting with Clippy (`cargo clippy`)
   - Unit tests (`cargo test --lib --bins`)
   - Integration tests (`cargo test --test '*'`)

---

## PR Management Workflow

The Makefile provides automated targets for managing Dependabot PRs:

### Available Targets

1. **`make list-dependabot-prs`** - List all open Dependabot PRs with their numbers, titles, and update times
2. **`make checkout-dependabot-pr PR=123`** - Checkout a specific PR branch
3. **`make test-dependabot-pr-full PR=123`** - Checkout and test a PR in one command
4. **`make merge-dependabot-pr PR=123`** - Merge a PR with squash and cleanup local branch
5. **`make merge-all-dependabot-prs`** - Test and merge all open PRs (use with caution!)

### Workflow Example

```bash
# Step 1: See what PRs are available
make list-dependabot-prs

# Step 2: Test a specific PR
make test-dependabot-pr-full PR=123

# Step 3: If tests pass, merge it
make merge-dependabot-pr PR=123

# Step 4: Return to main branch (automatic in merge target)
```

The merge target automatically:
- Squashes all commits into one
- Uses admin privileges to bypass branch protection rules
- Cleans up the local PR branch after merge
- Returns you to the main branch

---

## What the Target Does

The `make test-dependabot-pr` target performs a complete validation:

1. **Clean Build** - Removes all previous build artifacts to ensure a fresh build
2. **Compile** - Builds the project with the updated dependencies
3. **Format Check** - Verifies code formatting follows rustfmt standards
4. **Lint** - Runs Clippy to catch common mistakes and improve code quality
5. **Unit Tests** - Runs all unit tests in `src/`
6. **Integration Tests** - Runs all integration tests in `tests/`

All of these must pass before merging a dependency update.

---

## Troubleshooting

### Build Failures

If the build fails after a dependency update:

```bash
# Check what changed
cargo tree --duplicates

# Review dependency resolution
cargo tree -i <package-name>

# Try updating Cargo.lock
cargo update
```

**Common causes:**
- Breaking API changes in dependencies
- Incompatible version combinations
- Missing features that need to be enabled

**Resolution:**
- Review the dependency's CHANGELOG or release notes
- Update code to match new API if breaking changes occurred
- Consider pinning to an older version temporarily if urgent

### Format Check Failures

If `cargo fmt --check` fails:

```bash
# Auto-format all code
make fmt

# Or directly with cargo
cargo fmt

# Check what changed
git diff

# Commit formatting changes
git add -A
git commit -m "style: apply rustfmt"
git push
```

### Clippy Warnings/Errors

If Clippy reports new warnings or errors:

```bash
# See detailed output
cargo clippy -- -D warnings

# Fix automatically (where possible)
cargo clippy --fix

# Review changes
git diff
```

**Common scenarios:**
- New Clippy lints introduced in Rust toolchain updates
- Dependency updates expose existing issues
- API changes require code adjustments

**Resolution:**
1. Fix legitimate issues in code
2. Allow specific lints if false positive: `#[allow(clippy::lint_name)]`
3. Update code to use better patterns suggested by Clippy

### Test Failures

If tests fail after dependency updates:

```bash
# Run tests with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for specific module
cargo test --lib module_name
```

**Common causes:**
- Behavior changes in dependencies (especially test utilities)
- Timing issues with async code
- Mock/stub behavior changes

**Resolution:**
- Review test output carefully
- Check if dependency behavior changed (not a bug)
- Update test expectations if behavior change is acceptable
- Fix application code if tests caught a real issue

### Cargo.lock Conflicts

If you have merge conflicts in `Cargo.lock`:

```bash
# Accept the PR's version
git checkout --theirs Cargo.lock

# Regenerate lock file
cargo update

# Or rebuild from scratch
rm Cargo.lock
cargo build

# Commit resolved lock file
git add Cargo.lock
git commit -m "deps: resolve Cargo.lock conflicts"
```

---

## When to Close vs. Fix

### Close the PR if:
- The update introduces a breaking change we're not ready for
- Tests reveal a bug in the new dependency version
- The change requires significant refactoring
- Security advisory suggests waiting for a patch

### Fix the PR if:
- Formatting or linting issues are easy to fix
- Tests need minor adjustments for API changes
- Documentation needs updating

---

## Configuration

Dependabot is configured in `.github/dependabot.yml`:

### Update Schedule
- **Frequency**: Weekly (Monday at 9:00 AM ET)
- **Ecosystems**: Cargo (Rust), GitHub Actions

### Package Groups
Dependencies are grouped for logical updates:

1. **async-runtime** - Tokio and futures
2. **tui-framework** - Ratatui and Crossterm
3. **logging** - Tracing and related crates
4. **dev-tools** - CLI and error handling utilities

### PR Limits
- Maximum 5 open PRs per ecosystem
- Prevents overwhelming the team with updates

### Adjusting Configuration

To modify Dependabot behavior, edit `.github/dependabot.yml`:

```yaml
# Change update frequency
schedule:
  interval: "daily"  # or "monthly"

# Adjust PR limit
open-pull-requests-limit: 10

# Add new package group
groups:
  my-group:
    patterns:
      - "package-*"
```

After changes, Dependabot will use the new configuration for future PRs.

---

## Security Updates

Dependabot automatically creates PRs for security vulnerabilities.

**Priority**: Security updates should be reviewed and merged quickly.

```bash
# Check for security advisories
cargo audit

# Install cargo-audit if needed
cargo install cargo-audit

# Review the advisory
gh pr view <PR#>
```

Security PRs are labeled with `security` and should be fast-tracked.

---

## Rust-Specific Considerations

### Toolchain Updates

When `rust-toolchain.toml` or GitHub Actions updates Rust version:
- New Clippy lints may trigger
- Formatting rules may change slightly
- Compile times may vary
- New language features available

### Major Dependency Updates

For major version bumps (e.g., `tokio 1.x` → `tokio 2.x`):
1. Review the migration guide in the dependency's documentation
2. Test thoroughly - major versions often have breaking changes
3. Consider doing these separately from minor updates
4. May require code changes beyond what Dependabot provides

### Async Runtime Updates

Updates to `tokio` or other async runtime crates:
- Test with actual TCP connections (manual testing)
- Verify performance hasn't regressed
- Check compatibility between `tokio` and `tokio-util` versions

---

## Best Practices

1. **Review Release Notes**: Check what changed before merging
2. **Test Locally**: Run `make test-dependabot-pr` for non-trivial updates
3. **Group Related Updates**: Let Dependabot group related packages
4. **Monitor CI**: Trust CI, but verify security-sensitive changes
5. **Keep PRs Fresh**: Merge or close stale PRs to avoid conflicts

---

## Useful Commands

```bash
# List all open Dependabot PRs
make list-dependabot-prs

# View specific PR details
gh pr view <PR#>

# Checkout and test PR
make test-dependabot-pr-full PR=<PR#>

# Merge a tested PR
make merge-dependabot-pr PR=<PR#>

# Check dependency tree
cargo tree

# Update all dependencies to latest compatible versions
cargo update

# Run only quick checks
make static

# Full CI suite
make ci
```

---

## References

- [Dependabot Documentation](https://docs.github.com/en/code-security/dependabot)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [Rust Security Advisories](https://rustsec.org/)
- [GitHub CLI Documentation](https://cli.github.com/manual/)

---

## Questions?

If you encounter issues not covered here, please:
1. Check CI logs for detailed error messages
2. Review the dependency's changelog/release notes
3. Ask in team chat or open a discussion
4. Update this document with what you learned!
