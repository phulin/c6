# c6

A Rust foundation for an exact, deterministic, headless Civilization VI simulator. The intended interface is state import, externally supplied actions, turn advancement and state export, with behavior checked against the original GameCore DLL.

**This is an early implementation, not a playable Civilization VI engine.** It currently reproduces isolated gameplay RNG and coordinate primitives. Full player actions, end-turn execution and Civ VI save import are unsupported.

| Area | Current support |
| --- | --- |
| Gameplay `GameCore::Random` | Draw, initialize, reset, lock and unlock logical state; original-DLL differential checks with logging disabled |
| Coordinates | Plot sentinel validity and isolated plot-distance arithmetic |
| State transport | Versioned simulator envelope, RNG state, binary/configuration identities and lossless opaque sections |
| External loading | Rust API and versioned C shared-library API in [`include/c6.h`](include/c6.h) |
| Original-DLL testing | Linux x86-64 process oracle and identity-pinned differential corpus |
| World, players, units, cities, combat and economy | Unsupported |
| Legal gameplay actions and complete turn progression | Unsupported |
| Civ VI save loading and complete game snapshots | Unsupported |

Opaque state retention does not imply that those sections can be simulated. Current RNG coverage excludes native logging and synchronization callbacks. The coordinate validity primitive checks a sentinel, not map bounds.

## Build and check

Run from the repository root with Rust/Cargo installed:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p c6-ffi
```

On Linux, the resulting loadable library is `target/release/libc6_ffi.so`. With a C compiler available, exercise the public C API:

```sh
bash tools/check/c_abi_smoke.sh
```

The workspace separates the dependency-free simulation kernel (`c6-core`), shared-library boundary (`c6-ffi`) and process-based validation (`c6-differential`). JSON belongs to the test transport, not the action hot path. See [architecture and state format](docs/architecture.md).

## Compare with original machine code

These checks require Linux x86-64, Python 3, Make, a C compiler and a separately supplied original Gathering Storm FinalRelease DLL. The default location is `../c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll`. Its required SHA-256 is:

```text
324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c
```

```sh
make -C native/oracle
python3 tools/oracle/test_native.py
cargo build -p c6-differential --bins
python3 fixtures/generate.py
target/debug/c6-differential fixtures/corpus/xp2-rng.json fixtures/run-native-oracle target/debug/c6-rng-candidate
```

The generator records input cases and the actual candidate executable identity; expected outputs come from executing original DLL methods. Regenerate after rebuilding the candidate, then retain that corpus with its test result when recording evidence. Missing or mismatched binaries and unsupported operations cannot count as parity passes. The native host provides synthetic object fields and a disabled logging service; it does not initialize a game. See [oracle setup and limitations](docs/oracle.md) and [differential protocol](docs/differential.md).

## Research and next work

The [status and milestones](docs/status.md) and [engine dependency roadmap](docs/engine-roadmap.md) describe the remaining work toward complete actions and turns. [Reference provenance](docs/reference.md) distinguishes upstream mapping leads from runtime evidence. The persistent Ghidra project lives in the separate private `phulin/c6-ghidra` repository; [Ghidra setup](docs/ghidra.md) documents the pinned local tools and MCP workflow. Game binaries and private project data are not distributed here.

```sh
cargo bench -p c6-core --bench rng
```

This benchmark measures only the verified RNG primitive. It provides no game-turn throughput or whole-game speedup claim; see [performance measurements](docs/performance.md).
