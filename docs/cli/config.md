# engram config

Manage global engram configuration (API keys, model overrides).

## Usage

```bash
engram config set <key> <value>
engram config get <key>
engram config list
engram config path
```

## Description

Manages settings stored in the global config file (`~/.config/engram/repos.toml`). This is where you configure API keys for LLM-powered summarization and model overrides.

Environment variables always take precedence over config file values.

## Subcommands

### set

Set a configuration value.

```bash
engram config set anthropic_api_key sk-ant-api03-...
engram config set summarize_model claude-sonnet-4-20250514
```

API keys are masked in output for security:
```
Set anthropic_api_key = sk-a...03-x
```

### get

Get a configuration value.

```bash
engram config get anthropic_api_key
```

### list

List all configuration values.

```bash
engram config list
```

When no values are set, shows available keys with descriptions:
```
No configuration values set.

Available keys:
  anthropic_api_key    Anthropic API key for LLM summarization
  summarize_model      Model override (default: claude-haiku-4-5-20251001)
```

### path

Show the path to the global config file.

```bash
engram config path
# Output: /Users/you/.config/engram/repos.toml
```

## Available Keys

| Key | Description | Env Var Override |
|-----|-------------|-----------------|
| `anthropic_api_key` | API key for LLM-powered summarization at import | `ANTHROPIC_API_KEY` |
| `summarize_model` | Model for summarization (default: `claude-haiku-4-5-20251001`) | `ENGRAM_SUMMARIZE_MODEL` |

## Resolution Order

For each setting, engram checks in order:
1. **Environment variable** (highest priority)
2. **Config file** (`~/.config/engram/repos.toml`)
3. **Built-in default** (lowest priority)

## Security

The config file is set to `0600` permissions on Unix (owner read/write only) since it may contain API keys.

## Examples

```bash
# Set up LLM summarization
engram config set anthropic_api_key sk-ant-api03-...

# Use a different model for summarization
engram config set summarize_model claude-sonnet-4-20250514

# Check current config
engram config list

# Verify where config is stored
engram config path
```

## See Also

- [init](init.md) -- Shows LLM summarize status during setup
- [import](import.md) -- Uses API key for LLM summarization
- [Importing Sessions](../guides/importing-sessions.md) -- LLM summarization details
