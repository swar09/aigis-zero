# Contributing to Aigis-Zero

Contributions are welcome! To maintain code quality and consistency, please follow these guidelines when contributing to Aigis-Zero.

## Code Quality Standards

We enforce a zero-warning policy. Before opening a pull request, ensure that your changes pass all format, lint, and test checks:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## How to Help

For non-trivial changes, please open an issue first to align on the approach. 

Changes to the following components touch the security-critical surface area of the system and warrant design discussion before implementation:
- **Agent**
- **Fleet-server authentication paths**
- **Isolation module**

## Branch Naming Conventions

Please use the following naming conventions for branches:
- `feat/<short-description>` for new features
- `fix/<short-description>` for bug fixes
- `chore/<short-description>` for dependency updates, tooling, and CI
- `agent/<short-description>` for agent-specific work

## Development Workflow

The `main` branch is the stable reference. Active development happens on feature branches and is merged via pull requests after review and CI check completion.
