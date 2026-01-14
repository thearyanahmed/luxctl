# Task Management

## lux task list

List all tasks for the active project.

```bash
lux task list [--refresh]
```

| Option | Description |
|--------|-------------|
| `--refresh`, `-r` | Fetch fresh task data from server |

Output shows:
- Task number
- Points available
- Completion status (✓ completed, ✗ failed, … in progress)
- Task title

## lux task show

Show details for a specific task.

```bash
lux task show --task <NUMBER|SLUG> [--detailed]
```

| Option | Description |
|--------|-------------|
| `--task`, `-t` | Task number (1, 2, 3...) or slug |
| `--detailed`, `-d` | Show full task description |

Examples:

```bash
# by number
lux task show --task 1

# by slug with full description
lux task show --task initialize-a-repository --detailed
```
