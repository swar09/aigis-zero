# CI Build Caching Strategy

Rust builds are notoriously slow — a clean workspace build can take 5–15 minutes in CI. This document outlines the caching strategy to keep CI runs under 2 minutes for incremental builds.

---

## 1. GitHub Actions Cache Layers

### Layer 1 — Cargo Registry & Index (`~/.cargo`)

Caches downloaded crate sources and the registry index so `cargo` doesn't re-download 200+ dependencies on every run.

```yaml
- uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
    key: cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      cargo-registry-
```

### Layer 2 — Compiled Dependencies (`target/`)

This is the big one. Caching `target/` means only changed crates recompile.

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: ". -> target"
    shared-key: "workspace"
    cache-targets: true
    cache-all-crates: true
```

> **Why `Swatinem/rust-cache`?** It handles cache invalidation intelligently — hashing `Cargo.lock`, `Cargo.toml`, rustc version, and target triple. It also prunes stale artifacts to keep cache size under GitHub's 10GB limit.

### Layer 3 — sccache (Optional, for larger teams)

For teams with many contributors, `sccache` provides a shared compilation cache backed by S3/GCS.

```yaml
- name: Install sccache
  run: cargo install sccache --locked
- name: Configure sccache
  run: |
    echo "SCCACHE_GCS_BUCKET=your-bucket" >> $GITHUB_ENV
    echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV
```

---

## 2. Optimised CI Workflow

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  CARGO_INCREMENTAL: 0
  CARGO_NET_RETRY: 10
  RUST_BACKTRACE: short
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Check & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "workspace"

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Check
        run: cargo check --workspace

  test:
    name: Test
    runs-on: ubuntu-latest
    needs: check
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_PASSWORD: testpass
          POSTGRES_DB: edr_test
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: "workspace"

      - name: Run tests
        run: cargo test --workspace
        env:
          DATABASE_URL: postgres://postgres:testpass@localhost:5432/edr_test

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2.0.0
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

## 3. Key Optimisations Explained

| Setting | Why |
|---|---|
| `CARGO_INCREMENTAL: 0` | Incremental compilation produces larger caches with diminishing returns in CI. Disabling it produces smaller, more cacheable artifacts |
| `RUSTFLAGS: "-D warnings"` | Fail on warnings in CI — catches issues early without slowing local dev |
| `CARGO_NET_RETRY: 10` | Retry flaky crate downloads instead of failing the whole run |
| `needs: check` on test job | Skip expensive tests if formatting/linting fails — fast feedback |
| `shared-key: "workspace"` | All jobs share the same cache — test job reuses compiled deps from check job |

## 4. Cache Size Management

GitHub Actions caches are limited to **10 GB per repository**. Rust `target/` dirs can grow large.

**Mitigation strategies:**
- `Swatinem/rust-cache` automatically prunes old artifacts
- Set `CARGO_INCREMENTAL=0` to reduce cache size by ~40%
- Cache key includes `Cargo.lock` hash — dependency updates naturally invalidate stale caches
- Separate cache keys per branch if needed: `shared-key: "${{ github.ref }}"`

## 5. Local Development — Speeding Up Builds

For local builds on your Mac:

```bash
# Use mold linker (Linux) or zld (macOS) for faster linking
# macOS:
brew install michaeleisel/zld/zld
export RUSTFLAGS="-C link-arg=-fuse-ld=/usr/local/bin/zld"

# Or use cranelift backend for debug builds (much faster codegen)
# Add to .cargo/config.toml:
# [profile.dev]
# codegen-backend = "cranelift"

# Use cargo-watch for auto-rebuild on save
cargo install cargo-watch
cargo watch -x 'check --workspace'
```

## 6. Expected Build Times

| Scenario | Estimated Time |
|---|---|
| Clean build (no cache) | 8–15 min |
| Cached build (deps only) | 1–3 min |
| Cached build (incremental, single crate change) | 20–45 sec |
| `cargo check` (cached) | 10–20 sec |
