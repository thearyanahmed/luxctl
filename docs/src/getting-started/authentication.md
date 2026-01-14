# Authentication

Lux requires authentication to track your progress on Project Lighthouse.

## Get Your Token

1. Go to [projectlighthouse.io](https://projectlighthouse.io)
2. Log in or create an account
3. Navigate to Settings → API Token
4. Copy your token

## Authenticate

```bash
lux auth --token <YOUR_TOKEN>
```

On success, you'll see a welcome message with your name.

## Verify Authentication

```bash
lux whoami
```

This shows your name, email, and stats (projects attempted, tasks completed, total XP).

## Token Storage

Your token is stored locally at `~/.config/lux/config.toml`. Keep this file secure.
