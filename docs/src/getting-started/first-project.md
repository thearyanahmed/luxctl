# Your First Project

## Browse Available Projects

```bash
lux project list
```

This shows all available projects with their descriptions and task counts.

## Start a Project

```bash
lux project start --slug build-your-own-git --workspace ./my-git
```

Options:
- `--slug` (required): The project identifier
- `--workspace`: Directory for your code (defaults to current directory)
- `--runtime`: Language runtime (go, rust, c, python) - auto-detected if not specified

## Check Project Status

```bash
lux project status
```

Shows your active project, workspace, runtime, and progress.

## View Tasks

```bash
lux task list
```

Lists all tasks with their point values and completion status.

## Run Your First Task

```bash
# by task number
lux run --task 1

# or by slug
lux run --task initialize-a-repository
```

## Stop Working on a Project

```bash
lux project stop
```

This clears your active project. Your progress is saved on the server.
