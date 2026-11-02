#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
C6_GHIDRA_PROJECT=$(realpath "${C6_GHIDRA_PROJECT:-$C6_ROOT/../c6-ghidra/projects/c6-linux.gpr}")
exec python "$C6_ROOT/tools/ghidra/server.py" --transport streamable-http --host 127.0.0.1 --port "${C6_MCP_PORT:-8765}" \
  --project-path "$C6_GHIDRA_PROJECT" --max-workers 2 "$@"
