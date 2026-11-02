#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
mkdir -p "$C6_ROOT/.tools/downloads"
command -v uv >/dev/null || { echo 'Install uv before running this script.' >&2; exit 1; }
uv python install 3.12.14
uv venv --python 3.12.14 "$C6_ROOT/.tools/ghidra-venv"
uv pip sync --python "$C6_ROOT/.tools/ghidra-venv/bin/python" "$C6_ROOT/tools/ghidra/requirements.lock"
if [[ ! -f "$GHIDRA_INSTALL_DIR/Ghidra/application.properties" ]]; then
  curl -fL --retry 2 https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_12.2_build/ghidra_12.2_PUBLIC_20261016.zip -o "$C6_ROOT/.tools/downloads/ghidra.zip"
  unzip -q "$C6_ROOT/.tools/downloads/ghidra.zip" -d "$C6_ROOT/.tools"
fi
if [[ ! -x "$JAVA_HOME/bin/java" ]]; then
  curl -fL --retry 2 https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.5%2B8/OpenJDK25U-jdk_x64_linux_hotspot_25.0.5_8.tar.gz -o "$C6_ROOT/.tools/downloads/jdk.tar.gz"
  mkdir -p "$JAVA_HOME"
  tar -xzf "$C6_ROOT/.tools/downloads/jdk.tar.gz" -C "$JAVA_HOME" --strip-components=1
fi
java -version
pyghidra-mcp --version
