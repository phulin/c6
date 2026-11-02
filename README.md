# c6

A Rust foundation for an exact, deterministic, headless Civilization VI simulator: import state, supply legal actions externally, advance turns and inspect/export complete state, with behavior checked against original game code.

**The supplied Linux XP2 build is the sole execution and acceptance target.** Its installed depot metadata matches Valve's public branch in the [release audit](docs/version-audit.md). Windows code and evidence are preserved as frozen historical work. See [target policy](docs/targets.md).

**This is not yet a playable Civ VI engine.** Native Linux context initialization, original localization/script providers, StartupGame return and cleanup work. A restored gameplay session, legal actions and full turns remain unverified. The Rust rules loader now matches actual Linux-native base exports. Isolated Linux RNG, normal RNG serialization and stateless coordinate primitives now pass native comparisons. Other mechanics still need Linux validation; historical Windows tests do not establish Linux equivalence.

| Area | Linux target status |
| --- | --- |
| Original native host/GameCore | Genuine loader, context initialization, StartupGame return, provider cleanup and handle closure verified |
| Localization | Original Base English bootstrap, dictionary lookup and shutdown verified |
| Saved configuration and game loading | Original configuration reader returns with explicit canonical differences; no restored playable session |
| Save/load transport | Genuine stream adapters under validation; no completed native gameplay Load/Save round trip |
| Linux RNG primitives | Original Random arithmetic/state, independent objects and normal Read/Write record match Rust; gameplay stream ownership/logging excluded |
| Linux coordinate primitives | Original sentinel validity and stateless hex distance match Rust; no map bounds, movement or pathfinding claim |
| Linux rules inputs | Two original base exports match Rust/Python schema, typed rows and file fingerprints; full XP2 activation pending |
| Rust API and C shared library | Explicit state/identity envelope, branching and isolated operations; Linux mechanics parity pending |
| Full world/entities, legal actions and turns | Unsupported |

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

## Linux-native rules input

Two independent original Linux base exports now match the immutable `c6-data` loader and independent Python fingerprints across every pinned table and typed row. The fixture contains base content only; it does not activate the saved XP2 mod or create GameCore definition caches. See [native rules exports](docs/linux-rules.md) and [Rust rules validation](docs/rules-data.md) for profile identities, exact counts and rejection checks. [Original ASL filesystem initialization](docs/linux-filesystem.md) now also produces the same base database bytes and typed fingerprints without the diagnostic Schema-only view. This is a separate measured native input profile, not full mod-framework activation.

With the private pinned exports available:

```sh
cargo run --release -p c6-data --example inspect_rules -- \
  linux-xp2-base ../c6-ghidra/evidence/linux-rules/case-run-1 \
  ../c6-ghidra/evidence/linux-rules/case-run-2 \
  > /tmp/c6-data-linux-native-parity.json
```

This checks rules data, not gameplay or turn throughput. [Saved configuration](docs/linux-saved-configuration.md) restores through original code but has recorded added-key/flag differences and pending user settings. The populated-base native Load probe reaches the stream end but remains inside Load until timeout; this is not successful consumption or restoration; full XP2 activation and configured-state restoration remain active work.

## Coverage and next work

[Implementation status](docs/status.md) tracks the remaining full-state/action/turn milestones. [Linux asset identities](docs/linux-assets.md), [reference reuse](docs/reference-reuse.md) and [reference provenance](docs/reference.md) distinguish observed facts from discovery leads. Symbols and DWARF help reconstruct the Linux ABI; each behavior still needs a Linux-native oracle and complete affected-state comparison.

Historical Windows RNG, geometry, message-body and rules-export evidence remains available for research. Those passing comparisons are not Linux mechanics coverage; Linux RNG/coordinate primitive and base rules parity are separately recorded above. Windows bootstrap commands are inactive under the current Linux-only direction.

The private research repository `phulin/c6-ghidra` now contains the persistent `c6-linux.gpr` project, with verified targeted decompilation/MCP access and a clean checksummed snapshot. This is targeted analysis, not completed whole-program DWARF analysis; [Ghidra setup](docs/ghidra.md) documents the tools. Proprietary binaries, raw research artifacts, extracted saves and generated databases remain outside this public repository. [Performance notes](docs/performance.md) measure implemented primitives/data operations, not full-game throughput.
