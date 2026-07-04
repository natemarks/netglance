#!/bin/bash
# Validate development environment setup

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║         netglance Development Setup Validation              ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
PASSED=0
FAILED=0
WARNINGS=0

check_command() {
    local cmd=$1
    local name=$2
    local required=$3

    if command -v "$cmd" &> /dev/null; then
        echo -e "${GREEN}✓${NC} $name installed"
        ((PASSED++))
        return 0
    else
        if [ "$required" = "true" ]; then
            echo -e "${RED}✗${NC} $name NOT installed (required)"
            ((FAILED++))
        else
            echo -e "${YELLOW}⚠${NC} $name NOT installed (optional)"
            ((WARNINGS++))
        fi
        return 1
    fi
}

check_file() {
    local file=$1
    local name=$2

    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $name exists"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗${NC} $name NOT found"
        ((FAILED++))
        return 1
    fi
}

echo "Required Tools:"
check_command "cargo" "Rust/Cargo" "true"
check_command "git" "Git" "true"
check_command "rustc" "Rust Compiler" "true"

echo ""
echo "Recommended Tools:"
check_command "gh" "GitHub CLI" "false"
check_command "gitleaks" "Gitleaks" "false"

echo ""
echo "Optional Tools:"
check_command "cargo-audit" "cargo-audit" "false"
check_command "cargo-deny" "cargo-deny" "false"
check_command "cargo-udeps" "cargo-udeps" "false"
check_command "cargo-bloat" "cargo-bloat" "false"

echo ""
echo "Configuration Files:"
check_file ".gitleaks.toml" "Gitleaks config"
check_file ".githooks/pre-commit" "Pre-commit hook script"
check_file ".github/workflows/ci.yml" "CI workflow"
check_file ".github/workflows/auto-tag.yml" "Auto-tag workflow"
check_file ".github/workflows/release.yml" "Release workflow"
check_file ".github/workflows/gitleaks.yml" "Gitleaks workflow"
check_file ".github/dependabot.yml" "Dependabot config"

echo ""
echo "Git Hooks:"
if [ -f ".git/hooks/pre-commit" ]; then
    echo -e "${GREEN}✓${NC} Pre-commit hook installed"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠${NC} Pre-commit hook NOT installed"
    echo "  Run: make install-hooks"
    ((WARNINGS++))
fi

echo ""
echo "Makefile Targets:"
if grep -q "^test-gitleaks:" Makefile; then
    echo -e "${GREEN}✓${NC} test-gitleaks target exists"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} test-gitleaks target NOT found"
    ((FAILED++))
fi

if grep -q "^static:" Makefile; then
    echo -e "${GREEN}✓${NC} static target exists"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} static target NOT found"
    ((FAILED++))
fi

if grep -q "^release:" Makefile; then
    echo -e "${GREEN}✓${NC} release target exists"
    ((PASSED++))
else
    echo -e "${RED}✗${NC} release target NOT found"
    ((FAILED++))
fi

echo ""
echo "Documentation:"
check_file "docs/SETUP.md" "Setup guide"
check_file "docs/RELEASE_PROCESS.md" "Release process"
check_file "docs/CHANGES_SUMMARY.md" "Changes summary"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "Results:"
echo -e "  ${GREEN}Passed:${NC}   $PASSED"
echo -e "  ${YELLOW}Warnings:${NC} $WARNINGS"
echo -e "  ${RED}Failed:${NC}   $FAILED"
echo ""

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Setup validation FAILED${NC}"
    echo ""
    echo "Required items are missing. Please:"
    echo "  1. Install missing required tools"
    echo "  2. Run this script again"
    echo ""
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}Setup validation PASSED with warnings${NC}"
    echo ""
    echo "Recommended actions:"
    if ! command -v gh &> /dev/null; then
        echo "  - Install GitHub CLI: brew install gh"
    fi
    if ! command -v gitleaks &> /dev/null; then
        echo "  - Install gitleaks: brew install gitleaks"
    fi
    if [ ! -f ".git/hooks/pre-commit" ]; then
        echo "  - Install pre-commit hook: make install-hooks"
    fi
    echo ""
    echo "See docs/SETUP.md for detailed instructions."
    echo ""
    exit 0
else
    echo -e "${GREEN}✓✓✓ Setup validation PASSED ✓✓✓${NC}"
    echo ""
    echo "Your development environment is fully configured!"
    echo ""
    echo "Next steps:"
    echo "  1. Run: make static"
    echo "  2. Run: make test"
    echo "  3. Start developing!"
    echo ""
    exit 0
fi
