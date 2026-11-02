# Native Linux context lifetime

The supplied Linux build is the sole execution and acceptance target for the remaining state/actions/turn work, as requested by the user. The [current-public audit](version-audit.md) matches all 20 installed depots to Valve's public release metadata. Linux observations are gated by the Linux host, XP2 and complete input profile identities. Existing Windows evidence is historical and does not establish Linux behavior; different platform layouts must not be transferred without proof.

The unmodified Linux `Civ6` executable can provide the real host symbols and static initialization needed by the unmodified `libGameCore_XP2.so`. `native/linux/host_probe.c` intercepts glibc `__libc_start_main`, forwards to the genuine function with the original initialization/finalization arguments and a replacement diagnostic main, then loads the complete GameCore with `RTLD_NOW`. No fake GameCore imports, reconstructed object layouts, ELF edits, Wine, window or display server are used.

The measured checkpoint creates and destroys three genuine original contexts, closes the GameCore handle and exits zero. This establishes a viable diagnostic process for the supplied Linux profile, **not an initialized gameplay session or exact actions/turn library**. The original host application main is skipped. Original CRT and ELF initialization still run; host App initialization, Steam/network initialization through application main, and application shutdown do not run. The probe uses `_exit` after GameCore teardown because the skipped App startup does not establish application shutdown preconditions. A successful `dlclose` reports handle closure; shared GNU unique symbols or loader references may keep some mappings resident.

## Reproduce

A native C compiler and the original Linux assets are required. Ubuntu 24.04 also needs genuine GL/OpenAL loader dependencies. They may be installed normally, or extracted into the ignored work directory without installing a GUI:

```sh
mkdir -p target/linux/debs target/linux/sysroot
cd target/linux/debs
apt-get download libgl1 libglx0 libglvnd0 libopenal1 libopenal-data libsndio7.0
cd ../../..
python3 - <<'PY'
from pathlib import Path
import subprocess
for package in Path('target/linux/debs').glob('*.deb'):
    subprocess.run(['dpkg-deb', '-x', str(package), 'target/linux/sysroot'], check=True)
PY
python3 tools/linux/runtime_probe.py --output evidence/linux/runtime-context-lifetime.json
```

`--assets` and `--sysroot` override their defaults. The runner checks exact hashes of the original host, XP2 and all bundled host dependencies against `tools/linux/runtime-gate.json` before loader inspection or execution. It records all resolved immediate and transitive native dependencies, compiler, adapter source and compiled preload hashes in the configuration identity. `--expected-configuration HASH` refuses an identity mismatch before original execution. Original assets are never modified. The original text placeholders named `libc++.so` and `libtbb.so` are not used: required SONAME files `libc++.so.1` and `libtbb.so.2` are genuine ELF files already present.

The child runs in `target/linux/runtime`, with inherited `LD_*`/`C6_*` hooks removed, a 30-second timeout and no call to original application main. The output must exactly match all expected lifecycle markers as well as exit zero to pass this diagnostic. Missing dependencies, loader errors, signals and incomplete context cycles fail. No gameplay operation is implemented by this probe; rules, DLC selection, configuration databases, map, players, engine services, save loading, legal actions and turns remain outstanding.

## Evidence and ABI

[evidence/linux/runtime-context-lifetime.json](../evidence/linux/runtime-context-lifetime.json) contains the successful native run and the complete configuration identity. This is evidence for the declared Linux profile only. Existing Windows results do not satisfy Linux parity requirements; each operation must be compared against this Linux target and its declared input profile.

The Linux host SHA-256 is `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`; XP2 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. The host is `ET_EXEC`, exports its symbols to the native loader and calls dynamically linked glibc `__libc_start_main`; GameCore has no `DT_NEEDED` entries. The host's genuine exports satisfy its undefined GameCore providers through normal dynamic resolution.

For this XP2 build, DWARF DIE `0x0cf610ad` identifies `DllCreateGameContext` at `0x2dcb980` as a no-argument function returning `GameCore::Interfaces::IContext1*`. The adapter preserves the pointer opaquely and passes it to `DllDestroyGameContext` at `0x2dcb990`; Linux x86-64 System V pointer calling conventions apply. The factory jumps to original `GameCore::Exports::Context::Create()` at `0x2c7c880`, which allocates and constructs the original context wrapper and registers its instance; destroy reaches original `Destroy(Context*)` at `0x2c7c8d0` for registry removal, destruction and deallocation. These addresses are evidence for this hash only. Raw Linux-profile ABI evidence stays private at `../c6-ghidra/evidence/linux/xp2-factory.txt` (SHA-256 `117df38a39a0e3bbce3f13eb17d534a2de774edadcd170528d9335fbb201b451`).

## Original context initialization and startup boundary

`--initialize` adds the [genuine host-services fixture](linux-host-services.md), compiled into the same preload with source hashes in its configuration identity. It constructs the actual App, binds its original singleton and a proven documents-directory field to `/home/phulin/c6/target/linux-runtime/userdata`, constructs real EngineUtility/script/logging/achievement providers, and initializes real configuration/gameplay database objects through their original parameterized methods. The explicit Linux base source profile gates all 110 files under `Base/assets/configuration/data` and `Base/assets/gameplay/data` against `tools/linux/runtime-content-gate.json`, including an exact inventory check. These method returns do not establish source consumption, active DLC/mods or game configuration. Actual native backups later showed zero configuration tables/rows and eight gameplay tables with zero rows: the current initialization fixture has not imported the supplied base rules.

```sh
python3 tools/linux/runtime_probe.py --initialize \
  --output evidence/linux/runtime-context-initialize-complete.json
python3 tools/linux/runtime_probe.py --startup \
  --output evidence/linux/runtime-startup-base-boundary.json
```

Original exports `Context::Initialize()` returns **void**; the adapter queries the original `IsInitialied()` method afterward. [runtime-context-abi.json](../evidence/linux/runtime-context-abi.json) records the matching DWARF declarations. The complete initialization checkpoint observes a false-to-true native context flag, original context destruction, two further factory/destruction cycles, genuine archive-factory/EngineUtility/App cleanup, database shutdown and GameCore handle close, all with exit zero. The original native archive factory singleton is constructed with `FAutoArchiveFactory::CreateInstance`; this addresses the initially observed null-provider teardown fault without a replacement registry.

The first `--startup` probe enters original StartupGame and reaches original WorldBuilder initialization, then faults in host localization lookup because the original localization dictionary has not been initialized. [runtime-startup-base-boundary.json](../evidence/linux/runtime-startup-base-boundary.json) records failure, not a passing startup or gameplay comparison. The private debugger trace is `../c6-ghidra/evidence/linux-runtime/startup-base-first-fault-gdb.txt`. Earlier preserved boundary reports record the original missing configuration database URI fault and the subsequent missing archive factory teardown fault.

The Linux-bundled `Base/assets/benchmark/saves/xp2benchmark.civ6save` is byte-identical to the separately recorded Windows-supplied benchmark (SHA-256 `a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8`, 7,348,717 bytes); [runtime-xp2-benchmark-identity.json](../evidence/linux/runtime-xp2-benchmark-identity.json) records the Linux source. This identifies the same fixture bytes in both distributions; any Linux experiment still needs its own Linux-profile identity gate. It does not establish Linux save compatibility or successful Load. No native Linux Load, initialized gameplay session, legal action or turn has completed.


## Native streams and subsequent save/load probes

`--stream-check` calls the host's genuine `Serialization::MemoryIStream` and `MemoryOStream` constructors and methods. Native input initial/end/reset offsets, zero/full-capacity writes, and a refused overflow write are checked. The overflow fixture compares both the complete original stream object's bytes and its backing buffer with their prior values. [runtime-memory-stream-boundaries.json](../evidence/linux/runtime-memory-stream-boundaries.json) records the successful native process. This checks the adapter boundary, not gameplay or Rust parity.

`--localization --startup` adds the genuine base-English localization profile before Context initialization. The original localization database/bootstrap, dictionary initialization, Pottery lookup, invalid-language rejection and dictionary shutdown have run successfully. The first such probe advanced beyond WorldBuilder, then faulted in `GameCore::ScriptSystem::CreateLuaThread` because the genuine `LuaSystem::LuaScriptSystem3` singleton had not yet been started. [runtime-startup-localization-boundary.json](../evidence/linux/runtime-startup-localization-boundary.json) preserves that failed boundary.

`--startup --localization --lua` constructs the original host AppLuaScriptSystem singleton and verifies original Lua thread creation and a Havok numeric stack roundtrip. Original StartupGame then returns, and original Context/localization/Lua/App teardown plus handle close complete with exit zero. [runtime-startup-lua-boundary.json](../evidence/linux/runtime-startup-lua-boundary.json) records that successful diagnostic. It does not initialize map, players or a gameplay session.

`--load-stream PRIVATE_PAYLOAD` prepares a real native input stream and requests original Context Load after StartupGame. It implies the localization, Lua and stream checks. The runner requires the exact Linux XP2 benchmark source identity and the extracted payload SHA-256 `a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa` (76,556,938 bytes). The configuration records whether separate saved-configuration restoration was requested. Startup must return before this adapter enters Load. Matching DWARF establishes a four-byte signed `GameCore::Serializer::Result` enum with `OK=0` for original Save/Load. A passive crash observer records the original input-stream offset and fault instruction before failing the child; it supplies no game behavior.

`--save-output PRIVATE_PATH` optionally requests genuine Context Save after Load returns native OK. It uses the original output stream with an explicit 128 MiB capacity; existing output files and public repository destinations are refused. Save has not yet executed. The first original Load enters deserialization, consumes eight bytes and faults because the current rules fixture has no GameSpeed definition. [runtime-original-load-base.json](../evidence/linux/runtime-original-load-base.json) and the private debugger trace `../c6-ghidra/evidence/linux-runtime/load-first-fault-gdb.txt` preserve this failure. Neither an OK return nor retained output bytes alone establishes complete initialized game state or turn equivalence.


`--saved-configuration EXACT_LINUX_SAVE` calls the genuine host `Serializer::Manager::LoadConfiguration` through the separate configuration provider before StartupGame. `--saved-configuration-output PRIVATE_PATH` requests native GameConfig writeback for independent semantic comparison. The runner hashes the whole fixture and adapter, and reports the original restoration result separately from initialized gameplay state.

`--rules-snapshot-dir PRIVATE_NEW_DIRECTORY` calls original Linux `Database::Connection::BackupTo`, compares all source/destination table row counts and records actual output hashes. Two exports of the current initialization fixture are byte-identical, with zero configuration tables and eight empty gameplay tables. These establish the native backup boundary and expose the missing content import; they are not populated rules snapshots or evidence of gameplay parity. Concurrent runner executions hold a process lock through compilation and original execution so a preload cannot be replaced while another child maps it.


`--rules-source-view PRIVATE_VIEW` verifies an exact symlink mapping of the same 110 Linux files, adapting only the source `schema` directory spelling to `Schema`, as required by the original Linux Bootstrap literal. The original asset tree and SQL/XML bytes are unchanged; original application filesystem setup has not been recreated. With this explicit input profile, original Bootstrap generates populated base databases: configuration 78 tables/1,280 rows and gameplay 318 tables/23,966 rows. Two independent original backups pass all source/destination row-count comparisons under the same configuration identity. [runtime-rules-source-view-cause-probe.json](../evidence/linux/runtime-rules-source-view-cause-probe.json) and [runtime-rules-source-view-repeat.json](../evidence/linux/runtime-rules-source-view-repeat.json) preserve those runs.

The first native Load with this populated base profile and genuine saved-configuration restoration enters original Load, then exceeds the 30-second process bound. [runtime-original-load-base-populated.json](../evidence/linux/runtime-original-load-base-populated.json) records failure; no successful Load or further byte-consumption claim follows from the timeout. Full XP2 content activation and its original host context ownership are still being recovered.

A separate bounded debugger observation reads the original input stream cursor at 76,556,938 bytes while Load remains unreturned. [runtime-load-populated-hang-observation.json](../evidence/linux/runtime-load-populated-hang-observation.json) identifies the private trace and observation method. Reaching the payload end could mean valid state was consumed or that reading exhausted/misaligned the fixture; neither interpretation is established. The raw stack without original GameCore symbols/CFI does not establish later callers.
