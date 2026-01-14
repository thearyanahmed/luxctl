# Hints

Hints help you when you're stuck on a task. Unlocking a hint costs XP points.

## lux hint list

List available hints for a task.

```bash
lux hint list --task <NUMBER|SLUG>
```

Shows each hint with:
- Hint ID
- Point cost to unlock
- Whether it's already unlocked
- Hint text (if unlocked)

## lux hint unlock

Unlock a hint for a task.

```bash
lux hint unlock --task <NUMBER|SLUG> --hint <HINT_ID>
```

| Option | Description |
|--------|-------------|
| `--task`, `-t` | Task number or slug |
| `--hint`, `-i` | Hint UUID to unlock |

Example:

```bash
# list hints first
lux hint list --task 1

# unlock a specific hint
lux hint unlock --task 1 --hint abc123-def456
```

## Hint Strategy

- Hints are unlocked progressively - you may need to attempt the task before hints become available
- Point deductions are permanent for that task
- Try to solve tasks without hints for maximum XP
