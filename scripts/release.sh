#!/bin/bash
set -e

VERSION=$1

if [ -z "$VERSION" ]; then
    echo "error: version argument required"
    echo "usage: ./scripts/release.sh <version>"
    exit 1
fi

if ! echo "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$'; then
    echo "error: invalid version format. use semver (e.g., 0.2.0)"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
rm -f Cargo.toml.bak

sed -i.bak "s/pub const VERSION: &str = \".*\";/pub const VERSION: \&str = \"$VERSION\";/" src/lib.rs
rm -f src/lib.rs.bak

cargo check --quiet
cargo fmt -- --check
cargo clippy --quiet
cargo test --quiet

LUX_ENV=RELEASE cargo build --release

echo "release v$VERSION built: target/release/lux"
