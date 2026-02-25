# Testing

## Test Counts

| Suite | Count | Command |
|-------|-------|---------|
| Rust | 147 | `cargo test --workspace` |
| Python | 10 | `cd sdks/python && python3 -m pytest tests/ -v` |
| TypeScript | 7 | `cd sdks/typescript && npx vitest run` |
| **Total** | **164** | |

## Running Tests

### All Rust Tests

```bash
cargo test --workspace
```

### Single Crate

```bash
cargo test -p engram-core
cargo test -p engram-capture
cargo test -p engram-query
cargo test -p engram-protocol
cargo test -p engram-sdk
cargo test -p engram-mcp
```

### Python SDK

```bash
cd sdks/python
pip install -e ".[dev]"
python3 -m pytest tests/ -v
```

### TypeScript SDK

```bash
cd sdks/typescript
npm install
npx vitest run
```

## Test Patterns

### Temporary Git Repositories

Most tests that interact with Git create temporary repositories:

```rust
// Rust: use tempfile::TempDir
let tmp = TempDir::new().unwrap();
let repo = Repository::init(tmp.path()).unwrap();
// Configure user for commit signatures
let mut config = repo.config().unwrap();
config.set_str("user.name", "Test").unwrap();
config.set_str("user.email", "test@example.com").unwrap();
```

```python
# Python: use tmp_git_repo fixture (conftest.py)
def test_something(tmp_git_repo):
    storage = GitStorage.open(tmp_git_repo)
```

### Test Data Helpers

Each crate has `make_test_data()` helpers for creating engrams:

```rust
fn make_test_data(request: &str, files: &[&str], tokens: u64) -> EngramData {
    // Returns a fully populated EngramData for testing
}
```

### Float Comparisons

Use epsilon comparison for floating-point values:

```rust
assert!((cost - 0.10).abs() < 1e-10);
```

```python
assert data.manifest.token_usage.cost_usd == pytest.approx(0.02)
```

## What to Test

- **New CLI commands**: Add integration tests in the command module
- **New model fields**: Test serialization round-trip (serialize → deserialize → compare)
- **New storage operations**: Test in a temporary Git repo
- **New import formats**: Test with sample session data
- **Cross-SDK changes**: Ensure Rust, Python, and TypeScript serialize identically
