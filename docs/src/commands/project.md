# Project Management

## lux project list

List all available projects.

```bash
lux project list
```

## lux project show

Show details for a specific project.

```bash
lux project show --slug build-your-own-git
```

## lux project start

Start working on a project.

```bash
lux project start --slug <SLUG> [--workspace <PATH>] [--runtime <RUNTIME>]
```

| Option | Description | Default |
|--------|-------------|---------|
| `--slug`, `-s` | Project identifier (required) | - |
| `--workspace`, `-w` | Working directory for your code | `.` |
| `--runtime`, `-r` | Language runtime (go, rust, c, python) | auto-detect |

## lux project status

Show active project information.

```bash
lux project status
```

## lux project set

Update project settings.

```bash
lux project set --runtime go
```

## lux project stop

Stop working on the active project.

```bash
lux project stop
```
