# Simulator architecture and current boundary

The target is a deterministic, headless Civilization VI state machine driven by external actions: load a complete state, apply actions, advance turns, inspect the resulting state, and compare it against the original GameCore DLL. Native AI and rendering are outside that execution path. This repository currently implements a much smaller verified primitive. It does **not** implement a playable simulation, a complete save-state format, legal action enumeration, or end-turn execution.

## Executable layers

- `c6-core` is a dependency-free Rust library. `Engine` owns a `Snapshot`; `apply` accepts typed actions without JSON parsing or allocation for RNG actions. Unsupported operations return a typed error without changing state. A caller can clone a snapshot/engine to branch evaluation. There is no implicit global engine or random source.
- `c6-ffi` builds a shared library exposing the versioned `include/c6.h` API. An evaluation framework may load the platform library, check `c6_abi_version`, create/import handles, apply packed numeric actions, export state, and destroy handles before unloading. Each handle needs external synchronization. The ABI is native platform C, not a cross-machine wire ABI. Only validated live pointers are permitted.
- The differential harness and native oracle are separate processes. Their JSON protocol is a test transport; it does not sit in the simulation action hot path. The oracle calls original pinned DLL code. DLL subsystem dependencies must be explicitly recorded when stubbed.

## Coverage

| Subsystem | Current executable coverage |
|---|---|
| Gameplay random | `Get` arithmetic and logical counters; `Reset`; `Initialize`; lock count operations |
| RNG logging and synchronization callbacks | Excluded; no observable game-event equivalence claimed |
| Coordinate sentinel and plot distance | Isolated arithmetic primitives; no initialized map required |
| World, players, units, cities | Unsupported |
| Diplomacy, economy, combat | Unsupported |
| Turn progression and action legality | Unsupported |

The `COVERAGE` table exposes this boundary to Rust consumers. Its RNG entry means a verified primitive, not full equivalence of the owning subsystem. A future release should replace broad entries with individually versioned operation coverage as more routines become executable.

## Gameplay random

The primitive is **GameCore::Random**, not the separate UI cache PCG generator. Native oracle inspection of the final XP2 DLL (SHA256 `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`) identifies `Get` at RVA `0x5fa040`, `Initialize` at `0x5fa120`, and `Reset` at `0x5fa370`. See the native evidence directory for reproducible extraction and execution details. Reference data is pinned at commit `5af389445118bed2bdebf7a8a42b219c8fba5175`; Linux DWARF independently identifies seed/call/reset/lock/synchronous fields, but is not evidence that every Windows object has the same layout.

`Get` advances a 32-bit seed with wrapping multiplication by `0x41c64e6d` and addition of `0x3039`. It increments the 32-bit call counter and returns `((seed >> 16) * bound) >> 16`, where the bound and result are unsigned 16-bit values. Bound zero still advances state. `Reset` increments the wrapping reset counter and assigns seed while preserving calls and other fields. `Initialize` assigns seed, zeros calls/resets/lock count, and preserves the synchronous flag. `Lock` wraps its unsigned counter on increment; `Unlock` decrements only a positive counter. These logical operations do not implement host synchronization. `RandomState::new` denotes initialized asynchronous state, not the C++ constructor (whose reset counter initially differs).

The Rust `draw` signature makes the native effective bound explicit as `u16`. Test adapters casting raw native arguments must record both raw and effective bounds. The C API rejects an oversized draw argument so framework mistakes cannot be silently truncated. No global or host-language PRNG substitutes for this generator.

## Coordinates

`geometry::plot_valid` reproduces the sentinel check `x > -9999 && y > -9999`; it does not check actual map bounds. `plot_distance` reproduces the isolated XP2 routine at RVA `0x15ec0`, with width and wrap flag passed explicitly. Signed comparisons and wrapping arithmetic preserve native behavior even for extreme inputs; negative-row conversion has an `i32::MIN` corner case. Coordinate arithmetic does not establish world/map simulation coverage. See `evidence/engine/map-distance.asm` and the native differential fixtures.

## State and identity

`Snapshot` is a versioned simulator envelope. It carries logical RNG state, two 32-byte identity fields for the game binary and rules configuration, and a sorted map of named opaque byte sections. Unknown sections survive import, supported actions and export unchanged. This permits future state retention without inventing mechanics or discarding unknown data. It does not mean that unknown state can be simulated. Zero identity is an explicitly unbound primitive sandbox; consumers must require expected identities before evaluating game-state behavior. Loading actual Civ VI save files remains unsupported.

The canonical binary envelope is: `C6SN`; LE u32 version; 32 DLL hash bytes; 32 rules hash bytes; four LE u32 random fields (seed, calls, resets, lock count); one boolean byte (0/1); LE u64 section count; then repeated LE u64 UTF-8 name length/name bytes and LE u64 data length/data bytes. Sections emit in name order. Decoder rejects unknown versions, duplicates, invalid UTF-8/booleans, truncation and trailing data. Lengths are checked against available input before allocation. Transport version is distinct from ABI version and eventual game/rules schema version.

Full representation requires an audited field inventory for the exact DLL, rules database and enabled mods, including map, entity IDs, component containers, pending events, caches with semantic effects, turn scheduler state and every random stream. Add typed sections only after identifying their ownership and serialization semantics; preserve opaque sections until audited. Never call an envelope a complete game snapshot merely because it can store arbitrary bytes.

## Extending and validating

A new gameplay operation needs exact-version reference evidence, a native executable oracle with documented setup, boundary fixtures, and comparisons of both outputs and all affected state. Add support only after those comparisons pass; preserve failures as minimized reproducible fixtures. A full end-turn test must account for event order, active players, all random streams and every mutated subsystem. Current RNG tests cannot establish that equivalence.

`cargo test --workspace` checks core state and FFI contracts along with the harness tests. `cargo bench -p c6-core --bench rng` measures the actual verified RNG primitive using an optimization-resistant loop and emits a checksum and final state. It excludes JSON, logging and DLL host setup. It is not a game-turn throughput measurement, and no simulator speedup over Civ VI is inferred from it.
