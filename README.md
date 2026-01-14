# lux

CLI for [Project Lighthouse](https://projectlighthouse.io) - validate systems programming exercises locally.

## Install

```bash
# from source
cargo install --path .

# or build release binary
make release
./target/release/lux
```

## Usage

```bash
# authenticate
lux auth --token <TOKEN>

# show current user
lux whoami

# list projects
lux project list

# start a project
lux project start --slug tcp-echo-server --runtime go

# list tasks
lux task list

# show task details
lux task show --task 1

# run validation for a task
lux run --task 1

# validate all tasks
lux validate

# get hints (costs points)
lux hint list --task 1
lux hint unlock --task 1 --hint <HINT_UUID>
```

## Development

```bash
cargo build           # debug build
cargo test            # run tests
cargo fmt             # format
cargo clippy          # lint
make check            # all checks
```

### Release

```bash
make release:build VERSION=0.2.0
```
