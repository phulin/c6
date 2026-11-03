# c6

A Rust foundation for an exact, deterministic, headless Civilization VI simulator: import state, supply legal actions externally, advance turns and inspect/export complete state, with behavior checked against original game code.

The user-defined simulation boundary yields to external policies for **all player decisions**. The engine must reproduce exact gameplay state, gameplay RNG and rules/upkeep between decisions; native AI planning internals are outside scope only where proven planning-only. Gameplay-affecting diplomacy, deals and NPC state remain exact regardless of AI-related names. The [gameplay-only boundary](docs/linux-gameplay-oracle-scope.md) preserves rule validation and gameplay callbacks while excluding optional desktop/network integration; see also [scope and evidence rules](docs/targets.md#external-decision-policy).

**The supplied Linux XP2 build is the sole execution and acceptance target.** Its installed depot metadata matches Valve's public branch in the [release audit](docs/version-audit.md). Windows code and evidence are preserved as frozen historical work. See [target policy](docs/targets.md).

**The complete Rust state/action/turn engine is not yet implemented.** The native Linux harness now activates XP2 content, attaches an offline session and successfully loads/saves the benchmark Core state with full constructed-provider cleanup. A separately identified authoritative Core SkipTurn executes repeatably, but complete gameplay affected-state parity, branch-restorable state, general actions and turns remain unverified. Full desktop/UI/network admission is optional integration, not a completion requirement for this library. Rust supplies validated primitives, bounded state/serialization components and rules/registry infrastructure. It is not yet usable for complete-game AI evaluation.

| Area | Implemented and checked boundary | Missing for gameplay support |
| --- | --- | --- |
| Native lifecycle and rules | Genuine providers, XP2 Configure/Update, offline session, Core Load/Save and owned cleanup; pinned Base/XP2 rules exports | Branch-restorable complete state; raw Save→Load still differs |
| Native SkipTurn | Authoritative conditional Core command repeats with matching post-Save, quiescence and cleanup | Complete callbacks, affected state/RNG and general action/turn parity |
| Rust mirrored primitives | Random/normal records, stateless validity/distance, average damage and bounded loaded-sync randomized damage | Complete gameplay RNG inventory, combat and damage application |
| Rust mirrored state phases | Bounded SkipTurn Unit/Formation projection, typed Busy/CanMove and ordered handler predicate; owned World infrastructure | Coupled live-owner updates, complete state import and action effects |
| Rust movement arithmetic | Nine phases/accumulator/ZOC selection match 7,461 native queries and 67,149 phase states | Recorded-fact arithmetic only; complete world queries, legality and movement actions remain missing |
| Rust placement counts | 17,584 ordered native query rows and 1,262 definition/Formation bindings match | Computed handler results remain native-uncompared; full manager/action support absent |
| Rust source-backed yields | Plot GetYield, improvement composition and technology/civic bonus helpers reviewed against Linux bodies | Live query providers and initialized native numerical acceptance remain missing |
| Rust movement-world facts | Recorded world facts plus native-2 typed derivation: All 6,996 effective masks/Wonder/domain groups and presence/coastal results match within a bounded fresh read | Arbitrary Map history, overflow and complete world/action support absent |
| Typed wire transport | Complete current Map4/Plot6 and supported City51 records; bounded Unit/Formation/queues, Trade, RequirementSystem, notifications and Player components | Complete Player/roster import, runtime fixups and omitted live state |
| Supplied-input projections | Acquired technology/civic hash arrays and complete post-Validate global nodes can populate World providers; City/District lookup and initialization branches | Actual complete owner/provider capture and native rebuild acceptance |
| Direct Rust Core import | Accepted byte-zero profile reaches first Player.Trade at 5,191,963, including 35 Formations, 11 Cities, 72 Districts and 36 Units | All 43 Player entries remain unfinished; complete live City/Player state and Unit roster absent |
| Full Rust gameplay engine | `Engine::apply` rejects unsupported gameplay actions without mutation | Legal actions, complete mandatory continuation and external-decision turns |
| Optional desktop/network integration | Original host receive/loading deferral observed separately | Full-host admission remains unresolved; it is not the authoritative gameplay completion gate |

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

The [average-damage arithmetic helper](docs/linux-combat-kernel.md) matches 32,364 unique original parameter cases in debug and release, with zero mismatches under the exact target/libm/FP profile. Its reported 27.5 ns/call measures arithmetic only, excluding randomized damage, modifiers, combat execution, actions and turns; see the [independent review](docs/linux-combat-kernel-review.md).

[Randomized damage](docs/linux-combat-random.md) additionally matches 80 actual loaded Game-sync transitions. The observed RNG archive entry was already dirty in every case; first insertion, full RNG inventory and complete combat remain unverified.

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

The [conditional Core component](docs/linux-core-component.md) now queues, receives and executes a fresh native SkipTurn command in two isolated runs, with matching input/post-Core streams, worker quiescence and cleanup. It supplies explicit admission preconditions and does not reroute the rejected full-host request. Historical reporting wrappers had an int/void ABI defect and missed overloads. The [corrected reporting profile](docs/linux-action-events.md) now passes native control/execution/repeat with eleven balanced Send/return observations. [Passive reporting controls](docs/linux-reporting-effects-observation.md) now finish normally with captured cache data unchanged and all Save/command checks passing; complete transitive gameplay effects and getter purity remain unproven.

Original full-host delivery remains blocked by the proven loading predicate, but [desktop/network admission is optional integration](docs/linux-gameplay-oracle-scope.md). The gameplay library must still reproduce authoritative rule checks, request ordering, rules/upkeep and gameplay-affecting callbacks. The [bounded Rust SkipTurn state phase](docs/linux-skip-turn-kernel.md) matches actual Unit/Formation records. [Typed Busy/CanMove and ordered handler predicates](docs/linux-unit-operation-legality.md) now replace two supplied answers; full manager legality, live query providers and callbacks remain missing, so this is not complete action support. `Engine::apply` still rejects gameplay actions; general Game actions and EndTurn remain unsupported.

The [Unit/Formation/queue codecs](docs/linux-unit-codec.md) and [independent review](docs/linux-unit-codec-review.md) validate complete supported wire records in the current captures, not live reconstruction or universal handler/variant support. [Yield registry](docs/linux-definition-ids.md) and [Unit registry](docs/linux-unit-definition-ids.md) observations keep type hashes, native indices and database row IDs distinct; no generic rules-row-ID conversion is assumed. The [action review](docs/linux-action-review.md) and [delivery evidence](docs/linux-action-delivery.md) separate passing registration/transport controls from the unresolved Core receive boundary. The [conditional oracle contract](docs/linux-core-oracle-boundary.md) and its actual component results remain explicitly separate from full-host admission and full gameplay parity.

The [typed Map codec](docs/linux-map-codec.md) now roundtrips the complete 4,693,980-byte native Map4/Plot6 section, including six component schemas, with independent field/range checks. Both original streams contain the same Map. General nonempty component branches have source-derived tests but still lack corresponding native fixtures. The [accepted direct Rust Core profile](docs/linux-core-player-units-prefix.md) (`c2802c3`) derives its cursor from byte zero without Python offsets, traces or searches, through the first Player’s Treasury, Units and Eras. It consumes 5,191,963 bytes, including 35 Formations, 11 Cities, 72 Districts and 36 Units, and stops before Player.Trade. All 43 Player entries remain unfinished. Newer source compositions are ahead of this accepted profile; complete Player/Unit-roster import and live world reconstruction remain unsupported.

The [owned World and prerequisite helpers](docs/linux-world-yields-review.md) provide reviewed lookup/ownership contracts and source-backed technology/civic improvement bonus arithmetic over supplied typed inputs. The [prerequisite projection](docs/linux-player-prerequisites-review.md) installs acquired masks and complete live global-node availability; typed Techs/Culture wire codecs do not yet supply complete live provider reconstruction or initialized native numerical comparison.

The [Player component codecs](docs/linux-player-components.md) roundtrip all 86 captured Congress records. All native refund vectors are empty; nonempty refund coverage is source-derived. These are serialization and read-value rules, not Congress gameplay.

The [movement-world control](docs/linux-movement-world-control.md) accepts 1,536 adjacent edges in three fact groups from the initial truncated discovery. The [closed expanded movement comparison](docs/linux-movement-cost-expanded-results.md) now accepts **7,461 ordered queries and 67,149 native phase states**, with zero arithmetic mismatches. Expanded discovery reaches all 1,250 Formations while scanning 1,262 Units in this fixture, with zero skipped formations and no truncation; this is not all-game-case coverage. Twenty-four direct whole-Core comparisons each retain all 80,657,462 bytes, with zero differences or masked bytes. Both known Game/Map RNG objects remain unchanged within each process; complete gameplay RNG inventory is unresolved. This accepts query arithmetic and recorded observations, not complete state, movement legality/actions or turns. The earlier [180-second timeout](evidence/linux/movement-cost-timeout-1.json) remains preserved as failed history.

The [bounded fresh-read domain projection](docs/linux-movement-domain-default-review.md) matches all **6,996 effective full signed-16 masks, Wonder results and domain-query groups**, plus attribute presence/coastal results. Its 1,709 fallback rows are source-derived, with no unavailable effective query. Acceptance requires one fresh canonical Map read, positive consistent geometry, plot count `P` satisfying `2*P <= 65535`, and normal typed ownership without private-pool aliases. Arbitrary Map history, overflow and full allocator behavior remain unsupported. The unchanged native-2 capture retains six equal full Core Saves, both known RNG objects and cleanup; complete state/RNG, actions and turns remain incomplete.

The [native placement-count comparison](docs/linux-placement-count-results-review.md) matches **17,584 ordered count-position rows** (225 null, 17,359 present) and all **1,262 Unit definition/Formation bindings**, with genuine control/war/definition inputs. All 43 saved/live alive flags agree; six full Core Saves and the two known RNG objects are unchanged. The 1,262 computed SkipTurn handler results remain native-uncompared; a separately reviewed genuine handler helper has not been executed. The [placement benchmark](docs/linux-placement-query-benchmark.md) reports 117.16 ns median per combined lookup/layer/membership/count query on an EPYC 7642, excluding setup, actions, turns and native speed.

The [complete City wire review](docs/linux-city-wire-review.md) accepts 490 current records, 9,287,120 unmasked bytes and 5,880 child-owner boundaries within supported nested variants. This is exact transport for the captured records; live City initialization, Read fixups, omitted runtime state and gameplay remain unimplemented.

The [reviewed plot/improvement yield translation](docs/linux-plot-yield-review.md) implements outer GetYield and numerical composition over owned typed inputs. Real City/Player/property/visibility/climate providers and initialized native numerical parity remain absent. Source fixtures and byte audits do not turn these missing providers into zero-valued state.

The [prepared-fact movement benchmark](docs/linux-movement-cost-benchmark.md) records median 23.81 ns/edge without the ZOC prelude and 24.33 ns/edge with it. It excludes input capture, world queries, actions and turns; these are Rust arithmetic timings, not native movement latency.

[Captured notification controls](docs/linux-notification-captured-state-phase.md) also pass within their measured scope. Proven Unit EventStream entries/node IDs/allocation are rebuildable presentation output only in the [documented no-UI boundary](docs/linux-skip-turn-state-success.md); producer snapshot queries, properties and gameplay effects remain required.

The [RequirementSystem codec](docs/linux-requirements-codec.md), its [independent review](docs/linux-requirements-codec-review.md), and the [notification codec](docs/linux-notifications.md) add typed current-fixture serialization. Exact wire transport does not implement requirement evaluation, notification subscribers, live reconstruction or gameplay mechanics.

## Coverage and next work

[Implementation status](docs/status.md) tracks the remaining full-state/action/turn milestones. The pinned cru121 reference supplies concrete discovery leads, including 48,005 exact host name/address matches; this does not establish full GameCore identity or build age. [Linux asset identities](docs/linux-assets.md), [applied reference reuse](docs/reference-reuse.md) and [reference provenance](docs/reference.md) connect those leads to target-byte RE and original-code comparisons. Symbols and DWARF help reconstruct the Linux ABI; each behavior still needs a Linux-native oracle and complete affected-state comparison.

Historical Windows RNG, geometry, message-body and rules-export evidence remains available for research. Those passing comparisons are not Linux mechanics coverage; Linux RNG/coordinate primitive and rules-profile parity are separately recorded above. Windows bootstrap commands are inactive under the current Linux-only direction.

The private research repository `phulin/c6-ghidra` now contains the persistent `c6-linux.gpr` project, with verified targeted decompilation/MCP access and a clean checksummed snapshot. The latest closed local analysis batch has not been uploaded. This is targeted analysis, not completed whole-program DWARF analysis; [Ghidra setup](docs/ghidra.md) documents the tools. Proprietary binaries, raw research artifacts, extracted saves and generated databases remain outside this public repository. [Performance notes](docs/performance.md) measure implemented primitives/data operations, not full-game throughput.
