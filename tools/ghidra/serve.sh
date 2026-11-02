#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
exec python "$C6_ROOT/tools/ghidra/server.py" --transport streamable-http --host 127.0.0.1 --port "${C6_MCP_PORT:-8765}" \
  --project-path "$C6_ROOT/../c6-ghidra/projects/c6-xp2.gpr" --max-workers 2 "$@"
