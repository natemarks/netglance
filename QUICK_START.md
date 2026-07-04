# Quick Start Guide

## For Developers (First Time Setup)

```bash
# 1. Clone and enter repo
git clone https://github.com/nmarks/netglance.git
cd netglance

# 2. Install gitleaks (secret scanning)
brew install gitleaks  # macOS
# or: https://github.com/gitleaks/gitleaks#installing

# 3. Install git hooks
make install-hooks

# 4. Verify setup
./scripts/validate-setup.sh

# 5. Build and test
make static
make test
```

## Daily Development

```bash
# Run static checks before committing
make static

# Build debug binary
make build

# Run tests
make test

# Commit (pre-commit hook runs automatically)
git commit -m "Your changes"
```

## Release Process (Maintainers)

```bash
# 1. Create release PR
make release VERSION=0.3.0

# 2. Review and merge PR on GitHub

# 3. Tag is created automatically!
#    Monitor with: gh run watch

# 4. Done! Release builds automatically
```

## Common Commands

| Command | Description |
|---------|-------------|
| `make build` | Build debug binary |
| `make build-release` | Build release binary |
| `make test` | Run all tests |
| `make static` | Run all static checks (CI) |
| `make ci` | Run full CI suite |
| `make install-hooks` | Install gitleaks pre-commit hook |
| `make test-gitleaks` | Test gitleaks on entire repo |
| `make release VERSION=X.Y.Z` | Start release process |

## Troubleshooting

### Pre-commit hook fails
```bash
# Check what gitleaks found
gitleaks protect --staged --verbose

# If false positive, add to .gitleaks.toml allowlist
```

### Gitleaks not installed
```bash
# macOS
brew install gitleaks

# Linux
wget https://github.com/gitleaks/gitleaks/releases/latest/download/gitleaks_linux_x64.tar.gz
tar -xzf gitleaks_linux_x64.tar.gz
sudo mv gitleaks /usr/local/bin/
```

### Tests fail
```bash
# Clean and rebuild
make clean
make build
make test
```

## Documentation

- 📖 [Setup Guide](docs/SETUP.md) - Detailed setup instructions
- 📦 [Release Guide](docs/RELEASE_GUIDE.md) - Complete release process with examples
- 📝 [Changes Summary](docs/CHANGES_SUMMARY.md) - Implementation details
- ✅ [Implementation Checklist](IMPLEMENTATION_CHECKLIST.md) - Testing checklist

## Key Files

| File | Purpose |
|------|---------|
| `.gitleaks.toml` | Gitleaks configuration |
| `.githooks/pre-commit` | Pre-commit hook script |
| `.github/workflows/ci.yml` | CI workflow |
| `.github/workflows/auto-tag.yml` | Auto-tagging workflow |
| `.github/workflows/release.yml` | Release workflow |
| `Makefile` | Build and CI commands |

## Support

- Questions? Check [docs/SETUP.md](docs/SETUP.md)
- Issues? Open a GitHub issue
- Release questions? See [docs/RELEASE_PROCESS.md](docs/RELEASE_PROCESS.md)
