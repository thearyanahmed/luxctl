# luxctl

CLI for [projectlighthouse.io](https://projectlighthouse.io) — learn by building real systems.

## Install

```bash
# one-liner (installs Rust if needed)
curl -fsSL https://raw.githubusercontent.com/thearyanahmed/luxctl/master/install.sh | bash

# or via cargo
cargo install luxctl

# specific version
cargo install luxctl --version 0.5.3
```

## Quick Start

```bash
# authenticate with your token from projectlighthouse.io
luxctl auth --token $token

# verify setup
luxctl doctor

# see who you are
luxctl whoami
```

## Usage

```bash
# list available projects
luxctl project list

# start a project
luxctl project start --slug tcp-echo-server --runtime go

# list tasks for current project
luxctl task list

# show task details
luxctl task show --task 1

# run validation
luxctl run --task 1

# validate all tasks
luxctl validate

# get hints (costs points)
luxctl hint list --task 1
luxctl hint unlock --task 1 --hint $hint_uuid
```

## Development

```bash
cargo build           # debug build
cargo test            # run tests
cargo fmt             # format
cargo clippy          # lint
```

## License

MIT
