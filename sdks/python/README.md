# Engram Python SDK

Capture agent reasoning as Git-native versioned data.

## Installation

```bash
pip install engram
```

## Usage

```python
from engram import EngramSession

session = EngramSession.begin("my-agent", "claude-sonnet-4-5")
session.log_message("user", "Add OAuth2 authentication")
session.log_message("assistant", "Implementing OAuth2 with PKCE...")
session.log_tool_call("write_file", '{"path": "src/auth.rs"}', "Created auth module")
session.log_file_change("src/auth.rs", "created")
session.log_rejection("passport.js", "Middleware conflict with existing stack")
session.add_tokens(1500, 800, 0.02)

engram_id = session.commit("abc123", "Implemented OAuth2 with PKCE")
```

Or as a context manager:

```python
from engram import EngramSession

with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    session.log_message("user", "Add OAuth2 authentication")
    session.log_message("assistant", "Implementing...")
    # Automatically commits on exit
```
