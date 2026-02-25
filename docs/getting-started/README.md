# Installation

## Requirements

- **Rust 1.80+** and a C compiler (for vendored libgit2/OpenSSL)
- **Git** (any recent version)
- **Python 3.9+** (for Python SDK, optional)
- **Node.js 18+** (for TypeScript SDK, optional)

## Install the CLI

### From Source (recommended)

```bash
git clone https://github.com/AtticAIInc/Engram-SDK.git
cd Engram-SDK
cargo install --path crates/engram-cli
```

This installs the `engram` binary to `~/.cargo/bin/`. Make sure it's on your PATH:

```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export PATH="$HOME/.cargo/bin:$PATH"
```

### Verify Installation

```bash
engram version
```

## Install SDKs (optional)

SDKs let you integrate engram capture directly into your AI agent code.

### Python SDK

```bash
pip install engram
```

No compiled dependencies -- the Python SDK uses the git CLI via subprocess.

### TypeScript SDK

```bash
npm install @engram/sdk
```

### Rust SDK

Add to your `Cargo.toml`:

```toml
[dependencies]
engram-sdk = "0.1"
```

## Initialize in a Repository

After installing, initialize engram in any Git repository:

```bash
cd your-project
engram init
```

This sets up everything automatically:
- Git hooks (`prepare-commit-msg`, `post-commit`, `pre-push`) for commit trailers and auto-push
- Claude Code `SessionEnd` hook for auto-importing sessions on exit
- Auto-capture of agent sessions on commit
- Auto-push of engram refs when you `git push`
- Refspecs for syncing engram refs with remotes
- Git notes alias (`git loge`) for viewing reasoning on commits
- Search index directory at `.git/engram-index/`

All features are enabled by default. Opt out with `--no-auto-capture`, `--no-auto-push`, or `--no-claude-code`.

## Next Steps

- [Quick Start](quick-start.md) -- Record your first engram
- [Core Concepts](core-concepts.md) -- Understand the mental model
