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
