# Development Setup

This guide covers setting up your development environment for netglance.

## Prerequisites

### Required

- **Rust** (1.70 or later)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Git**
  ```bash
  # Usually pre-installed on macOS/Linux
  git --version
  ```

### Recommended

- **GitHub CLI** (for release process)
  ```bash
  # macOS
  brew install gh
  
  # Linux
  # See: https://github.com/cli/cli/blob/trunk/docs/install_linux.md
  
  # Authenticate
  gh auth login
  ```

- **Gitleaks** (for secret scanning)
  ```bash
  # macOS
  brew install gitleaks
  
  # Linux
  # Download from: https://github.com/gitleaks/gitleaks/releases
  wget https://github.com/gitleaks/gitleaks/releases/download/v8.18.2/gitleaks_8.18.2_linux_x64.tar.gz
  tar -xzf gitleaks_8.18.2_linux_x64.tar.gz
  sudo mv gitleaks /usr/local/bin/
  
  # Verify
  gitleaks version
  ```

### Optional

- **cargo-audit** (security vulnerability scanning)
  ```bash
  cargo install cargo-audit --locked
  ```

- **cargo-deny** (dependency policy checking)
  ```bash
  cargo install cargo-deny --locked
  ```

- **cargo-udeps** (unused dependency detection)
  ```bash
  cargo install cargo-udeps --locked
  rustup install nightly
  ```

- **cargo-bloat** (binary size analysis)
  ```bash
  cargo install cargo-bloat --locked
  ```

## Initial Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/nmarks/netglance.git
   cd netglance
   ```

2. **Install git hooks**
   ```bash
   make install-hooks
   ```
   
   This installs a pre-commit hook that runs gitleaks to prevent secrets from being committed.

3. **Verify setup**
   ```bash
   # Run all static checks
   make static
   
   # Build debug binary
   make build
   
   # Run tests
   make test
   ```

4. **Test gitleaks (optional)**
   ```bash
   # Scan entire repository for secrets
   make test-gitleaks
   ```

## Development Workflow

### Building

```bash
# Debug build (fast, includes debug symbols)
make build

# Release build (optimized)
make build-release

# Optimized build for native CPU
make build-optimized
```

### Testing

```bash
# Run all tests
make test

# Unit tests only
make unit-test

# Integration tests only
make integration-test
```

### Code Quality

```bash
# Format code
make fmt

# Check formatting (CI check)
make fmt-check

# Run linter
make clippy

# Basic linter (CI check)
make clippy-basic

# Check for dead code
make dead-code
```

### Static Analysis

```bash
# Run all static checks (format, lint, dead code, unit tests, audit)
make static

# This is what CI runs - make sure it passes before pushing!
```

### Security

```bash
# Audit dependencies for vulnerabilities
make audit

# Check dependency policies
make deny

# Find unused dependencies
make unused-deps
```

### Pre-commit Hook

The pre-commit hook automatically runs on every commit:

```bash
# Normal commit - hook runs automatically
git commit -m "Your changes"

# Skip hook (not recommended unless you're certain)
git commit --no-verify -m "Your changes"
```

**What the hook checks:**

1. **Gitleaks (Secret Scanning)**
   - Scans staged changes for secrets
   - Detects AWS keys, GitHub tokens, private keys, API keys, etc.
   - Prevents accidental commits of credentials

2. **Static Analysis (`make static`)**
   - Code formatting (`make fmt-check`)
   - Linting (`make clippy-basic`)
   - Dead code detection
   - Unit tests
   - Security audit (`cargo audit`)

**Note:** The hook runs the same checks that CI runs, catching issues before you push.

**If gitleaks detects a secret:**
1. Review the detection - is it a real secret?
2. If yes: Remove it and use environment variables or secure storage
3. If no (false positive): Add to `.gitleaks.toml` allowlist

## Common Tasks

### Run the application

```bash
# Debug build
cargo run

# Release build
cargo run --release

# With arguments
cargo run -- --help
cargo run -- --init-config
```

### Generate documentation

```bash
make doc
```

### Clean build artifacts

```bash
# Clean cargo build artifacts
make clean

# Clean everything including dist/
make clean-all
```

### CI/CD

```bash
# Run all CI checks locally
make ci

# This runs: make static + make integration-test
```

## Release Process

See [RELEASE_PROCESS.md](RELEASE_PROCESS.md) for detailed release instructions.

Quick overview:
```bash
# Create release PR
make release VERSION=0.3.0

# After PR is merged, tag is created automatically
# Monitor with: gh run watch
```

## Troubleshooting

### Gitleaks not found

```bash
# Install gitleaks
brew install gitleaks  # macOS

# Or download from GitHub releases (Linux)
wget https://github.com/gitleaks/gitleaks/releases/latest/download/gitleaks_linux_x64.tar.gz
tar -xzf gitleaks_linux_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
```

### Pre-commit hook fails

```bash
# Check what gitleaks found
gitleaks protect --staged --verbose

# Test on entire repo
make test-gitleaks

# Review .gitleaks.toml configuration
cat .gitleaks.toml
```

### Cargo audit fails

```bash
# Install cargo-audit
cargo install cargo-audit --locked

# Update vulnerable dependencies
cargo update

# Check for unfixable vulnerabilities
cargo audit
```

### Tests fail

```bash
# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run single integration test
cargo test --test integration_test_name
```

### Build fails

```bash
# Clean and rebuild
make clean
make build

# Check for compilation errors
cargo check

# Update dependencies
cargo update
```

## Editor Setup

### VS Code

Recommended extensions:
- `rust-analyzer` - Rust language server
- `CodeLLDB` - Debugger
- `Even Better TOML` - TOML syntax highlighting
- `crates` - Cargo.toml dependency management

### Vim/Neovim

```vim
" Install rust-analyzer via LSP plugin
" Example with coc.nvim:
:CocInstall coc-rust-analyzer
```

### IntelliJ / CLion

Install the Rust plugin from the marketplace.

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Gitleaks Documentation](https://github.com/gitleaks/gitleaks)
- [GitHub CLI Manual](https://cli.github.com/manual/)

---

**Questions?** Open an issue on GitHub.
