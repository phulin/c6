# Implementation status

The supplied Linux XP2 profile is the sole execution and acceptance target. The goal remains an exact headless Rust gameplay state/action/turn library, with external policies making **all player decisions** and exact rules/upkeep and gameplay RNG between decisions. [Desktop/UI/network admission is optional integration](linux-gameplay-oracle-scope.md), not a prerequisite to completing the authoritative gameplay library. Proven planning-only or presentation-only state may be omitted within its documented boundary; unknown gameplay effects remain required.

**The complete simulator is not implemented and is not yet usable for complete-game AI evaluation.** Original Linux Core loading and a conditional native SkipTurn execute repeatably. Rust has bounded mechanics/state phases, typed wire transport and native rules/registry components. Complete Player/Unit-roster import, live-world reconstruction, general Game actions, EndTurn and whole-world/RNG parity remain unsupported. No completion percentage is inferred from individual records or tests.

| Area | Accepted scope and remaining boundary |
| --- | --- |
| Original native lifecycle | Genuine filesystem, providers, XP2 Configure/Update, offline session, Core Load/Save and constructed-owner cleanup pass in identified profiles |
| Rules and registries | Base/activated-XP2 SQLite inputs match Rust/Python; actual Yield/Unit registries observed with explicit index/hash/row-ID distinctions |
| Map transport/import | Map4/Plot6 plus all six components; exact 4,693,980-byte native roundtrip, independent typed fields/ranges; structural Core-to-Map indexer works from byte zero |
| Remaining world import | Complete Player and Unit-roster indexing/live reconstruction absent; individual captured Unit/Formation records have bounded wire codecs |
| Native SkipTurn | Original queued Core command executes/repeats with matching post-Core output, worker quiescence and cleanup; no general action/turn parity |
| Rust SkipTurn | Complete observed Unit/Formation state projection matches; provider effects, omitted live state and broader aftermath incomplete |
| Reporting/notification controls | Corrected Send/return control/repeats and captured notification-state controls pass; passive4 captured data/Save/command checks pass, not global purity |
| Rust serialization | Bounded Unit/Formation/queues, Trade, RequirementSystem and notification components preserve supported typed records; they do not reconstruct every live owner |
| Mechanics primitives | Target-validated Random, stateless coordinates, average damage and bounded loaded-sync randomized damage; broader movement/combat and full RNG inventory unsupported |
| Desktop/network integration | Original host receive/loading gate remains unresolved; separate from required authoritative gameplay semantics |

## Typed Map milestone

The accepted [Map codec](linux-map-codec.md) consumes the complete current Map v4 / Plot v6 section, including Route, Feature, River, Terrain, Territories and TerritoryBuilder schemas. Every native byte, 2,208,846 independently decoded values, ten sections and 315 original nested boundaries match; the independent [review](linux-map-codec-review.md) also checks exposed nested ranges. The two canonical lifecycle streams contain identical Map bytes, not distinct map-generation coverage.

General nonempty Route/Terrain/Territory/localization forms have source-derived tests. Their populated native branches, Ice and wrapped-map original fixtures remain gaps. There is no opaque-component fallback, but typed transport does not implement Map::Read fixups, active-definition remapping, live world construction, movement or pathfinding.

The [Core stream indexer](linux-core-stream-layout.md) reaches Map structurally from raw Core byte zero, without native traces, fixture offsets or downstream Player schemas. It deliberately stops at the unsupported unframed Player boundary; independently extracted Unit/Formation records do not make a general roster importer. Movement-cost work is in progress, not accepted coverage.

## Gameplay callback and output boundary

The corrected native event profile repeats the original Core transition with eleven balanced Send/int-return calls per action. Actual reporting receivers include an active YieldMonitor with 6,996 plot records and 182 trade records. The accepted [passive4 observation](linux-reporting-effects-observation.md) finishes normally with full native/GDB cleanup, unchanged captured cache/PlotYields data and all five Save/command checks. Slot order is captured native order; route IDs/endpoints/paths are matched separately to parent records. Intermediate changes, unobserved pointees, all gameplay RNG and complete transitive getter effects are not certified.

[Captured notification controls](linux-notification-captured-state-phase.md) reproduce observed inputs within their bounded phase; they do not establish arbitrary subtype callbacks or complete notification gameplay. [Unit EventStream classification](linux-skip-turn-state-success.md) proves its queued entries, node IDs and allocator state are rebuildable output when reflected presentation delivery is omitted. Those UI output allocations are not remaining gameplay requirements. Source properties, producer snapshot queries, unrelated caches and any gameplay-affecting callbacks remain required; no namespace-wide exclusion follows.

The authoritative action API must preserve real owner/turn/phase legality, request normalization and ordering, complete mandatory continuation, and yield to the next external decision. Successful Core execution does not by itself complete that contract. Original full desktop load screens, platform-account startup and packet admission can remain optional integrations; no rejected packet is relabeled as accepted host ingress.

## Save and state comparison

The [normalization summary](../evidence/linux/save-normalization-summary.json) accounts for the raw 89,198-byte native Save→Load difference: 89,156 trade-order bytes with equal keyed payloads, and 42 GameEffects Team-ID/reference bytes following the observed +24 reconstruction map. Exact allocation sequencing, complete reference/RNG coverage and future ordered gameplay effects remain unproven. No bytes are masked or excluded as planning-only.

Native non-byte-idempotent loading is target behavior, not a defect to patch or a blanket block on scoped action comparisons. Same-source fresh-process native baselines repeat; exact simulator cloning must separately preserve complete imported logical gameplay state. One equal serialized sync-RNG record is not all-live-RNG coverage. Core Saves also omit live queues, jobs, some caches and other transient state.

## Next work

1. Extend structural import through complete Players, rosters and remaining gameplay systems, retaining identities, order, typed values and unsupported forms explicitly.
2. Finish authoritative SkipTurn callbacks/aftermath and complete affected-state/RNG comparisons, using the proven presentation-only exclusions rather than replicating native heap output.
3. Validate and implement movement, general Game actions and other rules with Linux-native fixtures; retain actual gameplay eligibility and ordered continuation.
4. Complete deterministic branching and full external-decision turn progression across every player and gameplay subsystem.
5. Measure initialization, memory and action/turn throughput only for implemented paths. Existing codec/data/arithmetic timings are not full-game benchmarks.

[README](../README.md) links active commands. Owner documents and [Linux target policy](targets.md) define exact profile identities and limits. Windows evidence remains frozen historical work and adds no Linux coverage.
