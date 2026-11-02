# Original DLL oracle

The oracle executes original x86-64 machine code from the supplied Gathering Storm DLL on Linux x86-64. It currently covers five isolated RNG methods and two geometry methods. It does **not** initialize a Civilization game, load a save, perform a gameplay action, or end a turn. Those requests return `unsupported` and cannot count as parity passes.

Build and validate from the repository root:

```sh
make -C native/oracle
python3 tools/oracle/run.py --identity
python3 tools/oracle/test_native.py
```

`run.py` reads protocol-v1 JSON objects, one per line, and writes one response per line. Supply `--dll PATH` to relocate the original binary. The default is `../c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll`. No game files or compiled native artifacts are committed. GCC or Clang and Python 3 are sufficient; Wine is unnecessary for these isolated methods.

The original DLL SHA256 must be `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`. A wrong or missing binary yields `unavailable`; a mismatched request identity yields `error`. The configuration identity is SHA256 of the exact bytes of `tools/oracle/config.json`, which pins both adapter and native-host source digests. The host embeds its source digest at build time; stale builds and adapter source changes are rejected. Native compiler/toolchain identity is not pinned. Obtain both identities with `--identity`; this describes the supported build and does not itself check that the local DLL exists.

The native host maps the PE sections at their preferred image base `0x180000000` using `MAP_FIXED_NOREPLACE`. It gives only `.text` executable permission and invokes methods with Microsoft's x64 calling convention (`ms_abi`). It does not bind imports, call `DllMain`, run static initializers, relocate pointers, or emulate a complete Windows process. Mapping fails if the preferred range is occupied. Never use this mapper with arbitrary binaries; its intentionally narrow RVA contract is enforced by the Python SHA256 gate.

The original RNG code queries a logging service through the data global at RVA `0xb8aa90`. The host supplies an explicit synthetic interface: vtable slot `0x50` returns null; initialization slots `0x40` and `0x28` do nothing. Original instructions remain unchanged. Thus this proves the RNG method's arithmetic and object mutations with logging disabled, including synchronous-flag inputs, rather than real logging/engine-service behavior. Geometry uses synthetic objects containing the exact fields read by the original methods. Synthetic state injection is deliberate and does not imply a complete game context.

| Method | RVA | Object contract |
| --- | --- | --- |
| `Random::Random` | `0x5f9ee0` | 32-byte object; vptr 0, seed 8, calls 12, resets 16, lock count 20, synchronous 24 |
| `Random::Get` | `0x5fa040` | `this`, unsigned 16-bit bound, nullable log label; returns unsigned 16-bit result |
| `Random::Initialize` | `0x5fa120` | `this`, unsigned 32-bit seed |
| `Random::Lock` | `0x5fa180` | `this` |
| `Random::Reset` | `0x5fa370` | `this`, unsigned 32-bit seed |
| `Random::Unlock` | `0x5fa380` | `this` |
| `Map::IInstance::GetPlotDistance` | `0x15ec0` | Synthetic map: signed width at `0x28`, wrap-X byte at `0x40`; four signed 32-bit coordinates passed by value |
| `PlotCoord::IsValid` | `0x315b80` | Two signed 32-bit coordinates, x at 0 and y at 4 |

The reference repository supplies candidate names and mapped addresses; the local DLL instructions supply the verified behavior and ABI. `evidence/native/xp2-random.asm` retains RNG disassembly; geometry disassembly is under `evidence/engine/`. PE summaries retain imports, exports, timestamps and format. Full verbose PE dumps remain locally as ignored `*-pe.txt` files. A PE timestamp is metadata, not a trusted product-version identifier; SHA256 is the version gate.

An RNG case provides `initial_state.rng` fields `seed`, `calls`, `resets`, `lock_count` (all u32), `synchronous` (bool). Omitted fields use original constructor values: 0, 0, 1, 0, false. Steps are `rng_get` with u32 `bound`, `rng_reset`/`rng_initialize` with u32 `seed`, or `rng_lock`/`rng_unlock`. Native Get truncates the bound to u16. Each response includes snapshots and Get traces identifying the original bound, effective bound and post-call u32 ordinal. `stream` is a caller trace label (default `sync`). Independent instances use numeric `stream_id` (u32, default 0); instance 0 is `initial_state.rng`. Supply additional states in `initial_state.rng_streams`, keyed by canonical nonzero numeric strings. Each instance is constructed by the original DLL. Every RNG snapshot includes all instance states as `rng` and, if the initial state included it, `rng_streams`. Unknown IDs are rejected before executing any steps. These instances establish independent Random-object behavior; assigning them to real engine subsystems requires separate evidence.

Geometry steps are `plot_distance` with signed i32 `width,x1,y1,x2,y2` and boolean `wrap`, or `plot_valid` with signed i32 `x,y`. Their snapshot is `{"result":...}` and their RNG trace is empty. Initial state can be `{}`. `plot_valid` only checks both coordinates exceed the sentinel -9999; it does not check map bounds.

`evidence/native/smoke-request.jsonl` and `smoke-response.jsonl` contain an actual original-DLL run. `test_native.py` additionally executes 832 RNG method calls across 64 cases covering u16 truncation, zero bounds, u32 seed/counter overflow, lock saturation behavior, initialize/reset differences and both logging flags. The Python expectation calculation checks recovered arithmetic against real DLL execution; it is not the native oracle. Rust differential tests compare independently implemented Rust operations with this native executable protocol.

Full action/end-turn validation still needs reconstruction of game-context interfaces, database/configuration initialization, game-state creation or loading, command dispatch, and externally visible snapshots. No unsupported operation is substituted with a toy simulator or accepted as a passing comparison.

The DLL is verified by Python and then opened by the native host. Inputs and compiled host are trusted local workspace files: concurrent replacement between hashing and loading is outside this harness contract.

The native validation also interleaves three independently constructed Random instances, confirms untouched seeds/counters/locks remain unchanged, and rejects an unknown stream ID without partial observations.

## Bounded context-interface probe and full-load prerequisites

`python3 tools/oracle/probe_context.py` executes the unchanged original `Exports::Context::QueryInterface` at RVA `0x5907a0`. It verifies the two accepted raw GUIDs and one unknown GUID. The method compares readonly constants and returns its supplied `this` identity or null without dereferencing that identity; the probe deliberately supplies a synthetic pointer. Output explicitly states `real_context_created: false`. This is interface ABI evidence, not factory execution, initialization, or game parity. Actual output and disassembly are retained as `evidence/native/context-query-response.json` and `context-query-interface.asm`.

The Windows factory export at RVA `0x752d50` jumps to `0x58fc60`; destroy at `0x752d60` jumps to `0x58fd10`. Factory code uses imported heap APIs, an instance registry, constructs a 16-byte wrapper, and allocates/constructs an underlying 0x1a80-byte XP2 object. The current isolated mapper intentionally does not execute this path. `context-factory.asm` records the original instructions. A genuine full Windows load needs a Windows runtime plus matching Database, Localization, HavokScript and VC runtime imports, static initialization and the engine utility interface before initialization. Wine/MinGW availability was inspected but no packages were downloaded while matching Linux assets were being requested.

For a native Linux route, acquire the matching `Civ6` executable, `libGameCore_XP2.so`, sibling shared libraries, data/configuration files and build metadata. Reference research identifies thousands of GameCore imports supplied by host-executable exports, so an isolated `.so` is insufficient for a reliable full bootstrap. Symbols/DWARF help reconstruct these interfaces; older symbol builds must retain separate hashes and are not interchangeable with this Windows target. Exact service interfaces, initialization order and state-loading behavior remain prerequisites before claiming full action/end-turn comparisons.

## Original RNG serialization

The separate `tools/oracle/serialization.py` JSONL oracle executes unchanged original `Random::Write` (`0x5fa390`) and `Random::Read` (`0x5fa190`). Its configuration and source identity are pinned independently in `serialization-config.json`; adding this probe does not change the gameplay/RNG-operation oracle contract. Build the native image loader with `make -C native/oracle`, then run:

```sh
python3 tools/oracle/serialization.py --identity
python3 tools/oracle/test_serialization.py
```

Each request includes `id`, `expected_engine` from this probe's `--identity`, and `op`. A `write` request supplies `state` with u32 `seed,calls,resets,lock_count` and u8 `synchronous_byte`; response `bytes_hex` contains the original serialization bytes. A `read` request supplies `bytes_hex` and an initial `state`; response contains restored `state`, `consumed:18`, and `trailing_hex`. Original Read leaves the initial lock count unchanged.

The normal record is 18 bytes: little-endian u32 constant version 1, seed, call count, reset count, raw synchronous byte, and zero legacy flag byte. Lock count is absent. Write emits version 1; Read consumes and ignores the version word, accepting zero and arbitrary other values. Both methods preserve arbitrary synchronous bytes, including 255. The Rust RNG state uses a canonical boolean, so its codec explicitly reports the unsupported representability case for raw bytes above 1. This is a Rust model limit, not a claim that the DLL rejects those bytes.

The probe uses synthetic in-memory stream interfaces as its I/O fixture. Original Write issues six virtual writes. Original Read supplies immutable 40-byte input request descriptors containing offset -1, requested length, alignment 16 and two null callback fields; the fixture checks these descriptors, copies bytes and advances its position. Each read is followed by virtual slot `0x40`, matching the actual MemoryIStream's no-op wait hook. The descriptors are not fabricated success results and are never mutated. Native callback counts verify consumption of exactly 18 bytes, even when trailing bytes remain.

Inputs shorter than 18 bytes and records whose final legacy flag is nonzero are rejected by the adapter before native execution. Original short-stream behavior depends on partial/uninitialized destination storage, and the legacy branch invokes additional vector reading and allocation code; neither is modeled or counted as parity. Original Read increments the existing reset count and zeros the seed before its first I/O call, but valid ordinary payload fields overwrite those values later. We make no claim about native error-path mutations without the real stream failure contract.

`test_serialization.py` executes 512 native write/read cases across all 256 raw synchronous bytes, arbitrary version words, initial lock preservation and prefix trailers, plus 19 unsupported-input guards. Actual example requests/results are retained in `evidence/native/serialization-request.jsonl` and `serialization-response.jsonl`. Detailed disassembly remains private under `../c6-ghidra/evidence/native/random-serialization.asm`.
