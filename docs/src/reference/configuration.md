# Configuration

## Config File Location

Lux stores configuration at:

- **Linux/macOS**: `~/.config/lux/config.toml`
- **Windows**: `%APPDATA%\lux\config.toml`

## Config File Format

```toml
[auth]
token = "your-api-token-here"
```

## State File

Project state (active project, cached tasks) is stored separately at:

- **Linux/macOS**: `~/.local/share/lux/state.json`
- **Windows**: `%LOCALAPPDATA%\lux\state.json`

The state file is integrity-protected using your auth token. Tampering with it will cause lux to reject it.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `LUX_LOG` | Set log level (error, warn, info, debug, trace) |

Example:

```bash
LUX_LOG=debug lux run --task 1
```
