# Ghidra analysis workspace

The persistent, private Ghidra repository is `../c6-ghidra`, remote `phulin/c6-ghidra`. The Linux project is `../c6-ghidra/projects/c6-linux.gpr`. The historical Windows project `c6-xp2.gpr` is frozen; do not analyze or modify it for Linux acceptance. It contains proprietary binary-derived data and must remain private. This simulator repository contains only tooling and concise observations.

Install with `tools/ghidra/install.sh` (requires uv, curl, unzip, tar). Versions: Ghidra 12.2 (`20261016`), Temurin JDK 25.0.5+8, Python 3.12.14, pyghidra-mcp 0.3.0. Python dependencies are locked in `tools/ghidra/requirements.lock`; binary download SHA-256s are recorded in the private research repository. Ghidra 12.2 requires JDK 25. Installation is local under ignored `.tools/`.

Start or reopen the server:

```sh
tools/ghidra/serve.sh
```

The local endpoint is `http://127.0.0.1:8765/mcp`. Do not open the same project in a second process. The launcher defaults to targeted access to saved functions; it leaves the Ghidra full-analysis flag unchanged (`false` until a full run completes). Set `C6_GHIDRA_AUTO_ANALYZE=1` to request whole-project analysis; whole-program semantic code indexing is disabled by default to keep shutdown practical (set `C6_GHIDRA_CODE_INDEX=1` to enable it); decompilation, symbol lookup, bytes, and cross-references work from the saved project. String/semantic search requires completed indexing. No original binary is modified. Set `C6_GHIDRA_PROJECT` to an explicit project path only when a different authorized project is needed.

```sh
source tools/ghidra/env.sh
python tools/ghidra/mcp_call.py list_project_binaries
python tools/ghidra/mcp_call.py decompile_function \
  '{"binary_name":"/libGameCore_XP2.so-614d46","name_or_address":"0x2d48d60"}' \
  --output ../c6-ghidra/evidence/linux/ghidra-random-get-decompiled.json
```

Use `list-tools` for the live tool schemas. Large outputs go into the private evidence directory. Only one process should mutate/save the project at a time. To close, send one SIGINT/Ctrl+C to the server and wait for its clean save-and-close path; a second interrupt abandons unsaved work. Snapshot only after clean close using `../c6-ghidra/scripts/snapshot-linux.sh`. Linux chunks and checksums live under `snapshots/linux/`; the historical Windows snapshot script/archive stay frozen.

Frozen historical Windows identities: FinalRelease SHA-256 is `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`; image base `0x180000000`. Release SHA-256 is `acd6cf2f352ae30ddf6f752bd7661165d9a5cf070f5519425ebab9ee0b865521`. Their RVAs and behavior must be established independently.

The Linux program identities are `/Civ6-96b377` (SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`) and `/libGameCore_XP2.so-614d46` (SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`). The host ELF image base is `0x400000`; GameCore is explicitly rebased to `0` using `import_targeted.py --image-base 0` (Ghidra initially chooses `0x100000` for this ET_DYN input). The import receipts record the actual analysis bases. Addresses below match original linked virtual addresses and GameCore module offsets, rather than inherited Windows RVAs.

`import_targeted.py --create-project` creates a missing private project and imports the fingerprinted input without running whole-program analysis. The default project for import, annotation and direct decompilation is `c6-linux`. Full DWARF import and semantic indexing are not prerequisites for bounded symbol-address decompilation. Source symbols remain evidence leads until validated against target bytes and native fixtures.

```sh
source tools/ghidra/env.sh
python tools/ghidra/decompile_project.py 0x2d48d60 0x2d48cf0 \
  --program libGameCore_XP2.so-614d46 --create-functions \
  --output ../c6-ghidra/evidence/linux/ghidra-random-targeted.json
```

The direct helper verifies the registered program SHA-256, exact function entry and first instruction. Optional function creation requires an existing imported symbol at an executable target address. It saves created boundaries and closes the exclusively owned project. It does not apply Linux structure layouts or establish complete subsystem parity. Check for a live owner before opening; stale lock files are not proof that a process still owns the project. Never remove an active owner's lock.

Official upstream: [Ghidra release](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.2_build), [PyGhidra MCP](https://github.com/clearbluejar/pyghidra-mcp).

For a closed project, `decompile_project.py` decompiles known addresses directly with PyGhidra and stores pseudo-C plus instruction listings. It refuses concurrent project ownership through the normal Ghidra lock. Target boundaries and annotations persist in the private full-project snapshots.

The frozen Windows project contains the exact host `CivilizationVI.exe-fd8d8f` (image base `0x140000000`) and `DatabaseDLL_FinalRelease.dll-5b8aae` (base `0x180000000`). Their identities are pinned in `program_identities.json`. `import_targeted.py` imports a fingerprinted binary with no eager whole-program analysis; close the MCP owner first. New identities must be registered before tools serve them.

The local server checks actual program SHA-256 before serving targeted operations. Numeric decompilation requires an exact function entry; disassembly requires the first defined instruction to equal the requested address. Upstream otherwise silently advances over undefined instructions or chooses a containing function. These errors require verified boundary creation or original-byte inspection; the wrapper rejects them instead of returning another location. Saved helper responses include request arguments and pinned program identity. Run `python tools/ghidra/test_address_guard.py` for the observed mismatch regression cases.

The frozen Windows project additionally includes exact `Localization_FinalRelease.dll-13175b` (SHA-256 `0becd5c425a3d6edc88f5315c01af3dd30dd7cb158271d56f4bba7ff2bb4e056`). `apply_annotations.py` applies private JSON plans only after checking every source hash and each target program SHA; it appends evidence comments, preserves existing meaningful names, and saves under exclusive project ownership. Native bootstrap evidence remains scoped to observed provider, parser, database, localization and stream boundaries; an initialized full-game Load/turn remains unproven.

Linux checkpoint validation: the persistent project contains both pinned ELF programs; targeted `UpdateContent`/`ConfigureContent` and `Random::Get`/`Initialize`/`Read`/`Write` have saved instruction bodies and bounded decompiles in private `evidence/linux/ghidra-*-targeted.json`. MCP list/decompile succeeds, and interior address `0x2d48d61` is rejected. Whole-program analysis, full DWARF type import, initialized state/action/turn equivalence and full save loading remain unproven.
