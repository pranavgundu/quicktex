#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo test --target "$(rustc -vV | sed -n 's/^host: //p')" "$@"
