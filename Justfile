# Justfile for Aigis-Zero EDR

# Run all pre-commit checks (fmt, clippy, typos, build, test)
check:
    ./scripts/check.sh

# Auto-fix formatting, clippy nits, and typos
fix:
    ./scripts/check.sh --fix

# Clean build artifacts and caches
clean:
    ./scripts/clean.sh

# Deep clean including cargo cache
clean-deep:
    ./scripts/clean.sh --deep

# Seed local development database
seed:
    ./scripts/seed.sh

# Reset and seed local development database
seed-reset:
    ./scripts/seed.sh --reset

# Run full CI check suite locally
ci:
    ./scripts/ci.sh

# Install developer tooling dependencies
setup:
    ./scripts/setup.sh

# Install git pre-commit hook
precommit:
    ./scripts/precommit.sh
