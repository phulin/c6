#!/usr/bin/env bash
# Source from any working directory. Local tool downloads are ignored by Git.
C6_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
export GHIDRA_INSTALL_DIR="$C6_ROOT/.tools/ghidra_12.2_PUBLIC"
export JAVA_HOME="$C6_ROOT/.tools/jdk-25"
export PATH="$JAVA_HOME/bin:$C6_ROOT/.tools/ghidra-venv/bin:$PATH"
