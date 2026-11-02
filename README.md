# c6

A Rust foundation for an exact, deterministic, headless Civilization VI simulator. The intended interface is state import, externally supplied actions, turn advancement and state export, with behavior checked against the original GameCore DLL.

The supplied Windows XP2 FinalRelease DLL is the existing validated behavioral baseline. The symbol-rich Linux build is a separate execution/research profile; its relative age and currentness remain under audit, and Linux may become the primary execution path. Coverage stays specific to each profile. See [binary profiles and version evidence](docs/targets.md).

**This is an early implementation, not a playable Civilization VI engine.** The Rust library currently reproduces isolated gameplay RNG, coordinate primitives and a unit-operation message-body format. Separate native bootstrap tools execute the original Windows DLL under Wine and have initialized real contexts with original database content. Full player actions, end-turn execution and Civ VI save import are unsupported.

| Area | Current support |
| --- | --- |
| Gameplay `GameCore::Random` | Draw, initialize, reset, lock and unlock logical state; original-DLL differential checks with logging disabled |
| Native RNG serialization | Original 18-byte normal `Random::Write`/`Read` payload; separate from the simulator snapshot envelope |
| Coordinates | Plot sentinel validity and isolated plot-distance arithmetic |
| State transport | Versioned simulator envelope, RNG state, binary/configuration identities and lossless opaque sections |
| External loading | Rust API and versioned C shared-library API in [`include/c6.h`](include/c6.h) |
| Action-message transport | Exact null-parameter unit-operation body codec; no action legality or execution |
| Original-DLL testing | Isolated Linux x86-64 oracles and identity-pinned differential corpora; separate whole-DLL Wine bootstrap |
| Native context bootstrap | Original StartupGame, context destruction and unload complete with verified providers and empty-only job-set cleanup |
| Native rules artifacts | Repeatable original Windows configuration/gameplay exports; immutable `c6-data` Rust loader verifies every pinned table and typed row |
| Native save probe | Original Load entered using the original borrowed MemoryIStream; no successful Load return or full-state capture |
| World, players, units, cities, combat and economy | Unsupported |
| Legal gameplay actions and complete turn progression | Unsupported |
| Civ VI save loading and complete game snapshots | Unsupported |

Opaque state retention does not imply that those sections can be simulated. Current RNG coverage includes independent caller-selected instances but excludes native logging and synchronization callbacks. The serialization adapters reject the legacy-vector branch and short input; raw synchronous bytes can round-trip as records, while applying values above 1 to logical Rust state is rejected. The coordinate validity primitive checks a sentinel, not map bounds.

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

The workspace separates the dependency-free simulation kernel (`c6-core`), immutable original-rules input (`c6-data`), shared-library boundary (`c6-ffi`) and process-based validation (`c6-differential`). JSON belongs to the test transport, not the action hot path. See [architecture and state format](docs/architecture.md).

## Compare with original machine code

These checks require Linux x86-64, Python 3, Make, a C compiler and a separately supplied original Gathering Storm FinalRelease DLL. The default location is `../c6-assets/DLC/Expansion2/Binaries/Win64/GameCore_XP2_FinalRelease.dll`. Its required SHA-256 is:

```text
324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c
```

```sh
make -C native/oracle
python3 tools/oracle/test_native.py
python3 tools/oracle/test_serialization.py
cargo build -p c6-differential --bins
python3 fixtures/generate.py
target/debug/c6-differential fixtures/corpus/xp2-rng.json fixtures/run-native-oracle target/debug/c6-rng-candidate
```

The generator records input cases and the actual candidate executable identity; expected outputs come from executing original DLL methods. Regenerate after rebuilding the candidate, then retain that corpus with its test result when recording evidence. Missing or mismatched binaries and unsupported operations cannot count as parity passes. The isolated native oracle provides synthetic object fields and a disabled logging service; it does not initialize a game. See [oracle setup and limitations](docs/oracle.md) and [differential protocol](docs/differential.md).

## Original Windows context bootstrap

A separate Wine/MinGW path loads the complete original GameCore DLL with `LoadLibraryW`, running its imports, TLS and static initialization. It creates genuine contexts and has completed original `Initialize`, the `IsInitialized` transition, destruction and DLL unload with real DatabaseDLL connections and declared original Base/XP2 SQL/XML content. Original host XML-import and hashing code supply the database fixtures, and the original embedded GameCore database phase contributes generated effect/type records; a separate original LocalizationDLL fixture resolves English text.

These are native context/data milestones. They do not provide a playable session, complete save load or a Rust gameplay implementation. Original StartupGame returns and context destruction/unload complete with genuine App/database/script providers and verified empty-only job-set cleanup. Load has entered original game-state code and records native stream progress, but has not returned successfully; the latest required service boundary is retained in the runtime evidence. A context initialized flag alone does not establish game readiness. Use the current commands, asset requirements and observations in [native runtime](docs/bootstrap-runtime.md), [integration checks](docs/bootstrap-validation.md), [host services](docs/bootstrap-services.md) and [save loading](docs/save-loading.md). The [native rules snapshots](docs/rules-snapshot.md) preserve actual Windows configuration/gameplay SQLite exports, reproduced byte-for-byte across independent builds. The immutable [Rust rules loader](docs/rules-data.md) matches their complete pinned schema and typed rows; these are rules inputs, not implemented gameplay. All executed modules and content inputs are fingerprinted; proprietary assets remain local.

The Rust [action-message codec](docs/actions.md) is checked against original serialization instructions. It encodes a request body, not a legal gameplay action. An initialized player, unit, registry and complete affected-state capture are still required before implementing SkipTurn.

## Research and next work

The [status and milestones](docs/status.md) and [engine dependency roadmap](docs/engine-roadmap.md) describe the remaining work toward complete actions and turns. [Reference provenance](docs/reference.md) distinguishes upstream mapping leads from runtime evidence. The persistent Ghidra project lives in the separate private `phulin/c6-ghidra` repository; [Ghidra setup](docs/ghidra.md) documents the pinned local tools and MCP workflow. Game binaries and private project data are not distributed here. For Linux bootstrap research, preserve the matching `Civ6` executable, `libGameCore_XP2.so`, dependencies and installation data; the [Linux inventory guidance](docs/engine-roadmap.md#incoming-linux-installation) describes the read-only inventory tool and the unresolved host contracts.

```sh
cargo bench -p c6-core --bench rng
```

This benchmark measures only the verified RNG primitive. It provides no game-turn throughput or whole-game speedup claim; see [performance measurements](docs/performance.md).
