# Running Validators

## lux run

Run validators for a specific task.

```bash
lux run --task <NUMBER|SLUG> [--project <SLUG>] [--detailed]
```

| Option | Description |
|--------|-------------|
| `--task`, `-t` | Task number or slug (required) |
| `--project`, `-p` | Project slug (uses active project if not specified) |
| `--detailed`, `-d` | Show task description before running |

## lux validate

Validate all tasks in the active project.

```bash
lux validate [--all] [--detailed]
```

| Option | Description |
|--------|-------------|
| `--all`, `-a` | Run all tasks, not just incomplete ones |
| `--detailed`, `-d` | Show task descriptions |

## How Validation Works

1. Lux reads the validator specifications for the task
2. Each validator runs against your code in the workspace directory
3. Results are displayed as pass/fail for each test case
4. On completion, results are submitted to Project Lighthouse
5. Points are awarded for first-time passes

## Example Output

```
validating...
✓ #1 can compile
✓ #2 tcp listening on port 8080
✗ #3 responds to GET request
  expected status 200, got 404
2/3 tests passed
```
