#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo build --release -p c6-ffi
check_dir=$(mktemp -d)
trap 'rm -rf "$check_dir"' EXIT
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -I include tools/check/c_abi_smoke.c \
    -L target/release -lc6_ffi -Wl,-rpath,"$PWD/target/release" -o "$check_dir/c_abi_smoke"
"$check_dir/c_abi_smoke"
