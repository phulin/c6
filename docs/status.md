# Implementation status

The supplied Linux XP2 profile is the sole execution and acceptance target. The goal remains an exact headless Rust gameplay state/action/turn library, with external policies making **all player decisions** and exact rules/upkeep and gameplay RNG between decisions. [Desktop/UI/network admission is optional integration](linux-gameplay-oracle-scope.md), not a prerequisite to completing the authoritative gameplay library. Proven planning-only or presentation-only state may be omitted within its documented boundary; unknown gameplay effects remain required.

**The complete simulator is not implemented and is not yet usable for complete-game AI evaluation.** Original Linux Core loading and a conditional native SkipTurn execute repeatably. Rust has bounded mechanics/state phases, typed wire transport and native rules/registry components. Complete Player/Unit-roster import, live-world reconstruction, general Game actions, EndTurn and whole-world/RNG parity remain unsupported. No completion percentage is inferred from individual records or tests.

| Boundary | What Rust mirrors or transports | Acceptance and remaining gap |
| --- | --- | --- |
| Original native lifecycle | Host providers, XP2 activation, offline session, Core Load/Save and constructed-owner cleanup | Passing identified native profiles; loading is not complete state import |
| Rules and registries | Pinned Base/activated-XP2 SQLite; explicit native index/hash/row-ID distinctions | Observed Yield/Unit and movement registries; no generic SQL-index substitution |
| Native-checked primitives | Random/normal records, stateless coordinates, average damage, bounded loaded-sync randomized damage | Scoped original-code comparisons; complete combat/RNG lifecycle absent |
| Bounded SkipTurn phase | Unit/Formation projection, typed Busy/CanMove and ordered handler predicate | Captured state matches; full manager legality, live query providers, callbacks and complete aftermath absent |
| Placement counts/bindings | Layered friendly counts with actual control/war/definition inputs | All 17,584 ordered native query rows and 1,262 bindings match; handler/full manager acceptance absent |
| Owned World | Map, ordered Players and optional complete Unit/Formation collections; native low-16-slot lookups | Source-reviewed ownership/query infrastructure; supplied records are not full Core import |
| Movement numerical functions | Values construction/finalization, nine phases, formation ignore aggregation, ZOC minimum selection | 7,461 recorded native queries and 67,149 phase states match; complete world queries and actions absent |
| Improvement prerequisite yields | GetPlayerBonusTechYieldFrom and civic analogue, ordered wrapping FixedPoint8 arithmetic | Source-reviewed supplied-input contract; initialized native differential acceptance absent |
| Movement-world facts | Unit/member/Formation facts, native definition identities, plot/edge queries | Initial facts and expanded arithmetic accepted; bounded fresh-read native-2 projection matches all 6,996 effective masks/Wonder/domain groups and presence/coastal results |
| Map wire import | Map4/Plot6 plus all six components, 4,693,980 native bytes | Exact current-fixture roundtrip and structural extraction; live Read fixups absent |
| Other wire components | Unit/Formation/queues, Trade, RequirementSystem, notifications, Player Goody_Hut/Bonuses/Congress | Supported current records only; complete owners and omitted live state absent |
| Prerequisite providers | Acquired bool hash arrays and complete post-Validate global-node projection into World | Supplied-input adapter; real complete owner/provider capture absent |
| City/District movement projection | Native ID/owner lookup and source-backed initial domain-mask rebuild branches | All 6,996 effective signed-16 masks/Wonder results match in a bounded fresh read; arbitrary history/overflow remains unsupported |
| Plot/improvement yields | Source-reviewed outer GetYield and numerical improvement composition, including corrected direct-index lookup | Live providers and initialized native numerical comparison absent |
| City wire composition | Complete current City51 records within supported nested variants | 490 native records / 9,287,120 unmasked bytes / 5,880 child boundaries accepted; live City state absent |
| Direct Rust Core prefix | Accepted byte-zero profile through first Player Units/Eras; 35 Formations / 11 Cities / 72 Districts / 36 Units | Stops at Player.Trade at 5,191,963; all 43 Player entries unfinished; full Unit roster absent |
| Reporting/notifications | Corrected Send/return controls and captured notification/passive4 state checks | Bounded captured effects; complete transitive callbacks/purity absent |
| Full state/actions/turns | No complete gameplay implementation | General `Engine::apply`, EndTurn, external-decision progression and whole-world/RNG parity unsupported |
| Desktop/network integration | Original loading/receive gate documented | Unresolved optional host admission, separate from authoritative gameplay semantics |

## Newly connected state and rules

The [owned World review](linux-world-yields-review.md) accepts the state assembly and two improvement prerequisite yield helpers as source-backed contracts. World owns actual ordered Map/Player/Unit/Formation inputs and preserves native lookup semantics; mutation alone does not maintain coupled occupancy, ownership or action effects. The technology/civic helpers retain native query order and wrapping FixedPoint8 arithmetic. Neither acceptance includes an initialized native numerical oracle.

The [prerequisite projection](linux-player-prerequisites-review.md) decodes already located acquired-bool arrays, applies the proven owner reset/remap rules and projects complete live post-Validate global nodes into World. Actual player mapping, native ordered registries and complete capture remain required. The accepted structural prefix now reaches Techs/Culture wire records, but their native Read fixups and live provider reconstruction remain separate work.

[Player Goody_Hut, Bonuses and Congress](linux-player-components.md) form part of the supported first-Player prefix. All 86 native Congress records roundtrip, with 6,182 independently reviewed leaves. Their refund vectors are empty; populated refund fixtures are source-only. Reset/overlay value semantics are implemented separately from omitted live ownership/archive state and Congress gameplay.

## Bounded movement progress

The [independently reviewed control3 comparison](linux-movement-world-control.md) binds the frozen world candidate to the exact original Core bytes and serializer trace containing one Map, 1,262 Units and 1,250 Formations. It matches all 1,536 actual adjacent edges in each of three fact groups, with zero mismatches or unsupported rows. Discovery was truncated at 256 formations; this is sample coverage, not complete world acceptance.

The [independently accepted bounded fresh-read projection](linux-movement-domain-default-review.md) matches all **6,996 effective full signed-16 masks, Wonder results and domain-query groups**, plus attribute presence/coastal results. All 1,709 fallback rows are now source-derived; no effective query is unavailable. This closes the captured native-2 getter comparison for one fresh canonical Map read, with positive consistent geometry, plot count `P` satisfying checked `2*P <= 65535`, and normal typed ownership/DTO use without private-pool aliases. Arbitrary history, overflow and full allocator behavior remain unsupported. The original native-2 profile/candidate remains immutable; six full Core Saves, both known RNG objects and cleanup are inherited controls. Complete world state, all gameplay RNG, later construction/transfer updates and actions/turns remain outside acceptance.

The [closed expanded native movement-cost comparison](linux-movement-cost-expanded-results.md) accepts all **7,461 ordered queries and 67,149 native phase states**, with zero arithmetic mismatches. Discovery reaches 1,250 Formations while scanning 1,262 Units, with zero skipped formations and no truncation. This covers the current fixture, not all game cases. All 24 whole-Core pairs compare 80,657,462 bytes directly, with zero differences and zero masked bytes. The full 32 raw bytes of both known Game/Map Random objects remain unchanged within each process; complete gameplay RNG inventory remains unresolved. This accepts arithmetic over original recorded facts and retained serialized observations. Complete world-query reconstruction, cache/provider effects, all gameplay RNG, legality, movement execution and turns remain outside acceptance.

The [first 180-second run](../evidence/linux/movement-cost-timeout-1.json) remains failed history: 588 complete rows without final Saves/RNG or cleanup. The successful 900-second rerun preserved its fixture, native calls, guards and frozen candidate. The [separate prepared-fact benchmark](linux-movement-cost-benchmark.md) reports median 23.81 ns/edge without ZOC and 24.33 ns/edge with ZOC. Input capture, world queries, actions and turns are excluded; instrumented native elapsed time is not a latency benchmark.

## Yield and City source progress

The [plot-yield review](linux-plot-yield-review.md) accepts the outer GetYield translation and improvement composition within their explicit typed-input/provider contract. It verifies first-match versus all-row behavior, wrapping FixedPoint8 arithmetic, source query order and the corrected direct-index improvement lookup. Live visibility, property, City/Player, adjacency and climate providers remain incomplete; no initialized native numerical GetYield parity is claimed.

The [complete City wire review](linux-city-wire-review.md) accepts 490 original City records, 9,287,120 unmasked bytes and 5,880 child-owner boundaries within the supported current nested-variant forms. The streams are a normalization pair from the same context, not independent gameplay branches. Native Read side effects, live owner/provider reconstruction, nonempty inherited-property native coverage and City gameplay remain absent.

## Typed Map milestone

The accepted [Map codec](linux-map-codec.md) consumes the complete current Map v4 / Plot v6 section, including Route, Feature, River, Terrain, Territories and TerritoryBuilder schemas. Every native byte, 2,208,846 independently decoded values, ten sections and 315 original nested boundaries match; the independent [review](linux-map-codec-review.md) also checks exposed nested ranges. The two canonical lifecycle streams contain identical Map bytes, not distinct map-generation coverage.

General nonempty Route/Terrain/Territory/localization forms have source-derived tests. Their populated native branches, Ice and wrapped-map original fixtures remain gaps. There is no opaque-component fallback, but typed transport does not implement Map::Read fixups, active-definition remapping, live world construction, movement or pathfinding.

The [accepted direct Rust byte-zero profile](linux-core-player-units-prefix.md) (`c2802c3`) reaches the first Player’s complete Treasury/Units and Eras, stopping before Player.Trade at **5,191,963**. Both originals reencode all **496,099 post-Map bytes / 70,280 items** identically. It includes 35 Formations, 11 Cities, 72 Districts and 36 Units; all 43 Player entries remain unfinished. Later source composition is ahead of this accepted profile. Full rosters, native Read fixups, live-world reconstruction and gameplay state/RNG remain incomplete.

## Gameplay callback and output boundary

The [Busy/CanMove and ordered SkipTurn handler predicates](linux-unit-operation-legality.md) remain bounded source contracts. The [independently accepted native placement comparison](linux-placement-count-results-review.md) now matches all **17,584 ordered count-position rows**: 225 null and 17,359 present counts, plus 1,262 definition/Formation bindings. Genuine native control/war/definition inputs and all 43 saved/live alive flags (35 true) are checked. Six full Core Saves match byte-for-byte; the known 64 RNG bytes and cleanup also pass. Logical membership does not reconstruct native allocator state.

The candidate computes 1,262 handler results (1,214 true, 48 false), but these remain **native-uncompared**. The separately reviewed genuine handler helper is prepared only; no original helper execution or full manager admission is accepted. Scripts, synchronous callbacks, mandatory aftermath and complete affected-state/RNG still block gameplay action support. The [placement benchmark](linux-placement-query-benchmark.md) measures all 17,359 present-position queries: combined lookup/layer/membership/count median **117.16 ns**, p10–p90 **114.89–125.77 ns**, on EPYC 7642. Setup, actions, turns and native speed are excluded.

The corrected native event profile repeats the original Core transition with eleven balanced Send/int-return calls per action. Actual reporting receivers include an active YieldMonitor with 6,996 plot records and 182 trade records. The accepted [passive4 observation](linux-reporting-effects-observation.md) finishes normally with full native/GDB cleanup, unchanged captured cache/PlotYields data and all five Save/command checks. Slot order is captured native order; route IDs/endpoints/paths are matched separately to parent records. Intermediate changes, unobserved pointees, all gameplay RNG and complete transitive getter effects are not certified.

[Captured notification controls](linux-notification-captured-state-phase.md) reproduce observed inputs within their bounded phase; they do not establish arbitrary subtype callbacks or complete notification gameplay. [Unit EventStream classification](linux-skip-turn-state-success.md) proves its queued entries, node IDs and allocator state are rebuildable output when reflected presentation delivery is omitted. Those UI output allocations are not remaining gameplay requirements. Source properties, producer snapshot queries, unrelated caches and any gameplay-affecting callbacks remain required; no namespace-wide exclusion follows.

The authoritative action API must preserve real owner/turn/phase legality, request normalization and ordering, complete mandatory continuation, and yield to the next external decision. Successful Core execution does not by itself complete that contract. Original full desktop load screens, platform-account startup and packet admission can remain optional integrations; no rejected packet is relabeled as accepted host ingress.

## Save and state comparison

The [normalization summary](../evidence/linux/save-normalization-summary.json) accounts for the raw 89,198-byte native Save→Load difference: 89,156 trade-order bytes with equal keyed payloads, and 42 GameEffects Team-ID/reference bytes following the observed +24 reconstruction map. Exact allocation sequencing, complete reference/RNG coverage and future ordered gameplay effects remain unproven. No bytes are masked or excluded as planning-only.

Native non-byte-idempotent loading is target behavior, not a defect to patch or a blanket block on scoped action comparisons. Same-source fresh-process native baselines repeat; exact simulator cloning must separately preserve complete imported logical gameplay state. One equal serialized sync-RNG record is not all-live-RNG coverage. Core Saves also omit live queues, jobs, some caches and other transient state.

## Next work

1. Extend structural import through complete Players, rosters and remaining gameplay systems, retaining identities, order, typed values and unsupported forms explicitly.
2. Finish authoritative SkipTurn callbacks/aftermath and complete affected-state/RNG comparisons, using the proven presentation-only exclusions rather than replicating native heap output.
3. Connect accepted movement arithmetic to complete world queries, validate movement legality/actions, and implement general Game actions with Linux-native fixtures and ordered continuation.
4. Complete deterministic branching and full external-decision turn progression across every player and gameplay subsystem.
5. Measure initialization, memory and action/turn throughput only for implemented paths. Existing codec/data/arithmetic timings are not full-game benchmarks.

[README](../README.md) links active commands. Owner documents and [Linux target policy](targets.md) define exact profile identities and limits. Windows evidence remains frozen historical work and adds no Linux coverage.
