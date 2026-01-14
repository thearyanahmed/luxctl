# Introduction

**Lux** is the CLI companion for [Project Lighthouse](https://projectlighthouse.io) - a hands-on systems programming learning platform.

## What is Lux?

Lux validates your code locally as you work through programming challenges. Instead of submitting code to a server, lux runs validators directly on your machine, giving you instant feedback.

## Features

- **Local validation** - Test your code instantly without waiting for remote servers
- **Multiple languages** - Support for Go, Rust, C, and Python projects
- **Progress tracking** - Track your progress across projects and tasks
- **Hints system** - Get help when you're stuck (costs XP points)

## Quick Start

```bash
# install lux
cargo install lux

# authenticate
lux auth --token <YOUR_TOKEN>

# start a project
lux project start --slug build-your-own-git

# see available tasks
lux task list

# run validators for a task
lux run --task 1
```

## Getting Help

- Run `lux --help` for command reference
- Visit [projectlighthouse.io](https://projectlighthouse.io) for tutorials
