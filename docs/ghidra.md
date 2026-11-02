# Ghidra analysis workspace

The persistent, private Ghidra repository is `../c6-ghidra`, remote `phulin/c6-ghidra`. The live project is `../c6-ghidra/projects/c6-xp2.gpr`. It contains proprietary binary-derived data and must remain private. This simulator repository contains only tooling and concise observations.

Install with `tools/ghidra/install.sh` (requires uv, curl, unzip, tar). Versions: Ghidra 12.2 (`20261016`), Temurin JDK 25.0.5+8, Python 3.12.14, pyghidra-mcp 0.3.0. Python dependencies are locked in `tools/ghidra/requirements.lock`; binary download SHA-256s are recorded in the private research repository. Ghidra 12.2 requires JDK 25. Installation is local under ignored `.tools/`.

Start or reopen the server:

```sh
tools/ghidra/serve.sh
```

The local endpoint is `http://127.0.0.1:8765/mcp`. Do not open the same project in a second process. Large program analysis runs asynchronously; whole-program semantic code indexing is disabled by default to keep shutdown practical (set `C6_GHIDRA_CODE_INDEX=1` to enable it); decompilation waits for Ghidra analysis, but does not require semantic indexing to finish. No original DLL is modified.

```sh
source tools/ghidra/env.sh
python tools/ghidra/mcp_call.py list_project_binaries
python tools/ghidra/mcp_call.py decompile_function \
  '{"binary_name":"/GameCore_XP2_FinalRelease.dll-15c1c1","name_or_address":"0x1805fa040"}' \
  --output ../c6-ghidra/evidence/random-get-decompiled.json
```

Use `list-tools` for the live tool schemas. Large outputs go into the private evidence directory. Only one process should mutate/save the project at a time. To close, send one SIGINT/Ctrl+C to the server and wait for its clean save-and-close path; a second interrupt abandons unsaved work. Snapshot only after clean close using the private repository's snapshot script.

The target FinalRelease SHA-256 is `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`; image base `0x180000000`. Release SHA-256 is `acd6cf2f352ae30ddf6f752bd7661165d9a5cf070f5519425ebab9ee0b865521`. Their RVAs and behavior must be established independently.

Official upstream: [Ghidra release](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.2_build), [PyGhidra MCP](https://github.com/clearbluejar/pyghidra-mcp).
