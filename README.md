# c6

A Rust foundation for an exact, deterministic, headless Civilization VI simulator: import state, supply legal actions externally, advance turns and inspect/export complete state, with behavior checked against original game code.

The user-defined simulation boundary yields to external policies for **all player decisions**. The engine must reproduce exact gameplay state, gameplay RNG and rules/upkeep between decisions; native AI planning internals are outside scope only where proven planning-only. Gameplay-affecting diplomacy, deals and NPC state remain exact regardless of AI-related names. See [scope and evidence rules](docs/targets.md#external-decision-policy).

**The supplied Linux XP2 build is the sole execution and acceptance target.** Its installed depot metadata matches Valve's public branch in the [release audit](docs/version-audit.md). Windows code and evidence are preserved as frozen historical work. See [target policy](docs/targets.md).

**The complete Rust state/action/turn engine is not yet implemented.** The native Linux harness now activates XP2 content, attaches an offline session and successfully loads/saves the benchmark Core state with full constructed-provider cleanup. Full host saved-load finalization, branch-restorable state, legal actions and turns remain unverified. Rust currently supplies validated primitives, bounded Unit/Formation/queue and Trade wire transport, immutable rules/registry components and state/FFI infrastructure.

| Area | Linux target status |
| --- | --- |
| Native host/context | Genuine filesystem, providers, StartupGame and owned cleanup verified |
| XP2 content | Original Configure/Update succeeds; diagnostic callback uses request None, censorship=false and constructor-only nongraphics UI |
| Linux rules inputs | Base and activated XP2 diagnostic exports match Rust/Python schema, typed rows and file fingerprints |
| Offline session | Original Attach/Leave and cleanup verified for Base and XP2 profiles |
| Native Core Load/Save | Benchmark Load returns OK; repeated Saves reproduce across processes; reloading captured output changes serialized bytes |
| Native loaded-state queries | Read-only game facade reports turn/phase/local-player/processing state without changing serialized output |
| Native action experiment | Controller/cache and no-action control pass; original Request/Network.Update attempt returns delivery failure without queueing or action execution |
| Full saved-game host lifecycle | RequestLoad ownership/reset verified; CheckRequest/Begin, PostInitialize, caches and active-user finalization incomplete |
| Rust wire/registry components | Current-fixture Unit/Formation/queues and Trade roundtrip; actual Yield/Unit registries observed; live reconstruction and gameplay unimplemented |
| Rust RNG/coordinates | Isolated Random/normal record and stateless validity/distance match original code; broader mechanics unimplemented |
| Full Rust state, legal actions and turns | Unsupported |

Unknown state sections can be retained, but cannot yet be simulated. A passing context lifecycle or localization lookup does not establish a complete game.

## Build and check

Run from the repository root with Rust/Cargo installed:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p c6-ffi
bash tools/check/c_abi_smoke.sh
```

The Linux library is `target/release/libc6_ffi.so`; its public header is [`include/c6.h`](include/c6.h). These checks verify implemented Rust/FFI contracts, not original Linux gameplay parity. The workspace separates `c6-core`, immutable rules input `c6-data`, the shared-library boundary `c6-ffi` and validation transport `c6-differential`. See [architecture](docs/architecture.md).

## Run the original Linux context

Use Linux x86-64, Python 3, a native C compiler, matching original assets under `../c6-assets-linux` and the genuine loader dependencies described in [Linux runtime setup](docs/linux-runtime.md). The runner gates the host, GameCore, dependencies, content and adapter configuration by identity.

```sh
python3 tools/linux/runtime_probe.py --output /tmp/c6-linux-context.json
python3 tools/linux/runtime_probe.py --startup --localization --lua \
  --output /tmp/c6-linux-startup.json
```

The target `libGameCore_XP2.so` SHA-256 is:

```text
a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa
```

The native host supplies its real exports and ELF initialization. Its application main is bypassed, so complete App startup, Steam/network state and normal application shutdown are not implied. Follow [current runtime observations](docs/linux-runtime.md) and [host-service contracts](docs/linux-host-services.md) for startup, configuration, stream and save probes. Run shared native probes serially.

## Compare isolated Linux primitives

```sh
python3 tools/linux/random_compare.py --output /tmp/c6-linux-random-parity.json
python3 tools/linux/geometry_compare.py --output /tmp/c6-linux-geometry-parity.json
```

This executes original Linux Random arithmetic/state operations, interleaved independent objects and normal Read/Write records against Rust. It excludes gameplay stream ownership/lifecycle, logging behavior, legacy/failing streams and turns. See [Linux RNG scope and evidence](docs/linux-random.md). The geometry comparison covers coordinate sentinel validity and stateless hex-distance arithmetic, excluding initialized maps, movement, pathfinding and turns; see [Linux geometry](docs/linux-geometry.md).

## Linux-native rules and Core state

Original XP2 ConfigureContent/UpdateContent now exports repeatable configuration/gameplay databases under a precisely identified diagnostic profile. The callback is genuinely constructed with request None and censorship=false; the UI provider has no graphics startup. This is not saved Request/Begin semantics. The immutable Rust loader verifies every pinned table and typed row; see [native activation](docs/linux-mod-activation.md), [rules validation](docs/rules-data.md) and [independent XP2 review](docs/linux-xp2-rules-review.md).

With the private pinned exports available:

```sh
cargo run --release -p c6-data --example inspect_rules -- \
  linux-xp2-activated-diagnostic \
  ../c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-1 \
  ../c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-2 \
  > /tmp/c6-data-linux-native-parity.json
```

This checks rules data, not gameplay or turn throughput. [Native Core Load/Save](docs/linux-game-load.md) now succeeds with the benchmark input and reproduces consecutive Saves across independent processes. Reloading those captured Saves produces a repeatable changed output, so byte-stable Save→Load→Save and complete branch restoration are **not** established. Accepted [typed classification](docs/linux-save-normalization.md) accounts for 89,156 bytes as trade record-order differences and 42 as GameEffects object/reference identity reconstruction. The exact allocation sequence, global reference coverage and observable ordering effects remain unresolved. Observable iteration effects remain unverified, so no bytes are masked and gameplay equivalence is not inferred. The [Rust trade codec](docs/linux-trade-codec.md) roundtrips these native wire records; it does not implement trade gameplay.

The [full saved-load lifecycle](docs/linux-save-lifecycle.md) still needs original request checking/Begin and host finalization. [Saved configuration](docs/linux-saved-configuration.md) also retains explicit canonical differences and pending-user semantics. [Action/turn contracts](docs/linux-actions.md) identify future execution boundaries; read-only native legality has passed for actual units, while mutating action/turn comparisons and full Rust mechanics remain unimplemented. Original controller/cache initialization and the no-action control now pass. The first Request reaches no queued message or action; the next task is diagnosing original request conversion versus broadcast/delivery gates. Encompassing host Begin/finalization remains a separate requirement.

The [Unit/Formation/queue codecs](docs/linux-unit-codec.md) and [independent review](docs/linux-unit-codec-review.md) validate complete supported wire records in the current captures, not live reconstruction or universal handler/variant support. [Yield registry](docs/linux-definition-ids.md) and [Unit registry](docs/linux-unit-definition-ids.md) observations keep type hashes, native indices and database row IDs distinct; no generic rules-row-ID conversion is assumed. The [action review](docs/linux-action-review.md) separates the passing control from the failed undelivered Request.

## Coverage and next work

[Implementation status](docs/status.md) tracks the remaining full-state/action/turn milestones. [Linux asset identities](docs/linux-assets.md), [reference reuse](docs/reference-reuse.md) and [reference provenance](docs/reference.md) distinguish observed facts from discovery leads. Symbols and DWARF help reconstruct the Linux ABI; each behavior still needs a Linux-native oracle and complete affected-state comparison.

Historical Windows RNG, geometry, message-body and rules-export evidence remains available for research. Those passing comparisons are not Linux mechanics coverage; Linux RNG/coordinate primitive and rules-profile parity are separately recorded above. Windows bootstrap commands are inactive under the current Linux-only direction.

The private research repository `phulin/c6-ghidra` now contains the persistent `c6-linux.gpr` project, with verified targeted decompilation/MCP access and a clean checksummed snapshot. This is targeted analysis, not completed whole-program DWARF analysis; [Ghidra setup](docs/ghidra.md) documents the tools. Proprietary binaries, raw research artifacts, extracted saves and generated databases remain outside this public repository. [Performance notes](docs/performance.md) measure implemented primitives/data operations, not full-game throughput.
