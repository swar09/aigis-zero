#!/usr/bin/env bash
set -euo pipefail

# scripts/setup.sh
# Cross-platform dependency installer for macOS and Linux.
# Installs system libraries, C/C++ build tools, Rust nightly toolchain components, and cargo tools.
# Usage: ./scripts/setup.sh

info()  { echo -e "\n\033[1;34m▶ $1\033[0m"; }
ok()    { echo -e "\033[1;32m✔ $1\033[0m"; }
warn()  { echo -e "\033[1;33m! $1\033[0m"; }

# 1. Detect OS and install native system dependencies
OS="$(uname -s)"
info "Detecting OS and installing system dependencies (OS: $OS)"

if [[ "$OS" == "Darwin" ]]; then
  if ! command -v brew >/dev/null 2>&1; then
    warn "Homebrew not found. Please install Homebrew from https://brew.sh/"
  else
    echo "Installing macOS dependencies via Homebrew..."
    brew install libpq openssl pkg-config protobuf cmake || true
  fi
elif [[ "$OS" == "Linux" ]]; then
  if command -v apt-get >/dev/null 2>&1; then
    echo "Installing Debian/Ubuntu dependencies via apt..."
    SUDO=""
    if [[ "$EUID" -ne 0 ]] && command -v sudo >/dev/null 2>&1; then
      SUDO="sudo"
    fi
    $SUDO apt-get update -y || true
    $SUDO apt-get install -y \
      build-essential \
      pkg-config \
      cmake \
      libssl-dev \
      libpq-dev \
      protobuf-compiler \
      libcurl4-openssl-dev \
      libclang-dev \
      curl || true
  elif command -v dnf >/dev/null 2>&1; then
    echo "Installing Fedora/RHEL dependencies via dnf..."
    SUDO=""
    if [[ "$EUID" -ne 0 ]] && command -v sudo >/dev/null 2>&1; then
      SUDO="sudo"
    fi
    $SUDO dnf install -y \
      gcc \
      gcc-c++ \
      make \
      pkgconf-pkg-config \
      cmake \
      openssl-devel \
      libpq-devel \
      protobuf-compiler \
      libcurl-devel \
      clang-devel || true
  elif command -v pacman >/dev/null 2>&1; then
    echo "Installing Arch Linux dependencies via pacman..."
    SUDO=""
    if [[ "$EUID" -ne 0 ]] && command -v sudo >/dev/null 2>&1; then
      SUDO="sudo"
    fi
    $SUDO pacman -Sy --noconfirm \
      base-devel \
      pkgconf \
      cmake \
      openssl \
      postgresql-libs \
      protobuf \
      curl \
      clang || true
  elif command -v apk >/dev/null 2>&1; then
    echo "Installing Alpine Linux dependencies via apk..."
    SUDO=""
    if [[ "$EUID" -ne 0 ]] && command -v sudo >/dev/null 2>&1; then
      SUDO="sudo"
    fi
    $SUDO apk add --no-cache \
      build-base \
      pkgconfig \
      cmake \
      openssl-dev \
      postgresql-dev \
      protobuf \
      curl \
      clang || true
  else
    warn "Unsupported Linux package manager. Please ensure libpq, openssl, protobuf, cmake, and pkg-config are installed."
  fi
fi

# 2. Rust Toolchain Configuration
info "Configuring Rust toolchains"
if command -v rustup >/dev/null 2>&1; then
  echo "Installing / updating nightly toolchain with rustfmt and clippy..."
  rustup toolchain install nightly --profile minimal --component rustfmt,cargo,clippy || true
else
  warn "rustup not found. Please install rustup from https://rustup.rs"
fi

# 3. Cargo Subcommand Tools
info "Installing cargo tool dependencies"
cargo install typos-cli --locked || true
cargo install cargo-audit --locked || true
cargo install cargo-cache --locked || true
cargo install sqlx-cli --locked || true

# 4. Script Permissions
info "Ensuring script execution permissions"
chmod +x scripts/*.sh 2>/dev/null || true
if [[ -d infra/scripts ]]; then
  chmod +x infra/scripts/*.sh 2>/dev/null || true
fi

ok "Setup complete — run ./scripts/check.sh to verify workspace health"

