# Building from Source

## Requirements

- **Rust 1.80+** (install via [rustup](https://rustup.rs/))
- **C compiler** (for vendored libgit2 and OpenSSL)
- **Git** (any recent version)
- **Python 3.9+** (for Python SDK tests)
- **Node.js 18+** (for TypeScript SDK tests)

## Build

```bash
source "$HOME/.cargo/env"        # Ensure cargo is on PATH
cargo build --workspace          # Build all 7 crates
```

## Lint

Zero warnings policy:

```bash
cargo clippy --workspace -- -D warnings
```

## Format

```bash
cargo fmt --all -- --check       # Check formatting
cargo fmt --all                  # Auto-format
```

## Run the CLI

```bash
cargo run -p engram-cli -- <command>

# Examples
cargo run -p engram-cli -- version
cargo run -p engram-cli -- log --cost
cargo run -p engram-cli -- search "authentication"
```

## Install Locally

```bash
cargo install --path crates/engram-cli
```

## SDK Development

### Python SDK

```bash
cd sdks/python
pip install -e ".[dev]"
```

### TypeScript SDK

```bash
cd sdks/typescript
npm install
npm run build
```
