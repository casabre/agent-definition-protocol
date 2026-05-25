#!/usr/bin/env bash
# Local CI validation — mirrors .github/workflows/ci.yml
# Run from the repo root. Rust llvm-cov skipped on macOS (needs llvm-tools-preview).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PASS=0; FAIL=0
step() { echo; echo "=== $* ==="; }
ok()   { echo "PASS: $1"; PASS=$((PASS+1)); }
fail() { echo "FAIL: $1 — $2"; FAIL=$((FAIL+1)); }

# ── lint-python ───────────────────────────────────────────────────────────────
step "lint-python: ruff"
if ruff check sdk/python; then ok "ruff"; else fail "ruff" "lint errors above"; fi

# ── test-python ───────────────────────────────────────────────────────────────
step "test-python: pytest + 100% coverage"
PYTHON=$(command -v python3 || command -v python)
# Use a venv if the system Python is externally managed (PEP 668)
VENV=/tmp/adp-cov-env
if [[ ! -d "$VENV" ]]; then
  "$PYTHON" -m venv "$VENV"
  "$VENV/bin/pip" install -q -e sdk/python pytest pytest-cov coverage
fi
if (cd sdk/python && "$VENV/bin/pytest" tests \
      --cov=adp_sdk --cov-report=term-missing --cov-fail-under=100 -q 2>&1); then
  ok "pytest 100%"
else
  fail "pytest" "coverage < 100% or test failures"
fi

# ── test-typescript ───────────────────────────────────────────────────────────
step "test-typescript: npm test + 100% coverage"
if (cd sdk/typescript && npm test -- --silent 2>&1); then ok "typescript"; else fail "typescript" "see above"; fi

# ── test-rust (cargo test only; llvm-cov requires Linux) ─────────────────────
step "test-rust: cargo test"
if (cd sdk/rust && cargo test --locked 2>&1); then
  ok "cargo test (all pass)"
  if [[ "$(uname)" == "Linux" ]] && command -v cargo-llvm-cov &>/dev/null; then
    step "test-rust: llvm-cov 100%"
    if (cd sdk/rust && cargo llvm-cov --all-features --locked --fail-under-lines 100 2>&1); then
      ok "llvm-cov 100%"
    else
      fail "llvm-cov" "coverage < 100%"
    fi
  else
    echo "  llvm-cov skipped (requires Linux + llvm-tools-preview — enforced in CI only)"
  fi
else
  fail "cargo test" "test failures above"
fi

# ── test-go ───────────────────────────────────────────────────────────────────
step "test-go: go test + 100% coverage"
if (cd sdk/go && go test ./... -coverprofile=coverage.out -covermode=atomic -count=1 2>&1); then
  COVERAGE=$(cd sdk/go && go tool cover -func=coverage.out | tail -1 | awk '{print $3}' | tr -d '%')
  echo "Go coverage: ${COVERAGE}%"
  if awk "BEGIN {exit !($COVERAGE >= 100)}"; then
    ok "go coverage 100%"
  else
    fail "go coverage" "${COVERAGE}% < 100%"
  fi
else
  fail "go test" "failures above"
fi

# ── validate-schemas ──────────────────────────────────────────────────────────
step "validate-schemas"
if bash scripts/validate.sh 2>&1; then ok "validate-schemas"; else fail "validate-schemas" "see above"; fi

# ── summary ───────────────────────────────────────────────────────────────────
echo
echo "────────────────────────────────────────"
echo "  PASSED: $PASS   FAILED: $FAIL"
echo "────────────────────────────────────────"
[[ $FAIL -eq 0 ]]
