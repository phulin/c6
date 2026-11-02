# Implementation status

The supplied Linux XP2 profile is the sole execution and acceptance target for the complete exact headless state/action/turn library. Windows work is frozen historical evidence. [Target policy](targets.md) records module identities and profile-specific validation.

The user has explicitly set the decision boundary: **all player decisions come from external policies**. Exact gameplay state/RNG and rules/upkeep remain required; only proven native AI planning internals are outside scope. AI-related gameplay data, diplomacy, deals and NPC state are not excluded by name. Unknown state remains unmasked. All 89,198 raw roundtrip byte differences now have a bounded classification, while complete live-state and behavioral equivalence remain unproven.

**Native XP2 Core state loading now works under an identified diagnostic profile. The complete Rust simulator remains unimplemented.** Native read-only action legality now runs on actual loaded units. Full-host saved-load finalization, branch-restorable state, mutating action execution and complete turns remain unverified.

| Deliverable | Verified scope and remaining boundary |
| --- | --- |
| Native host/providers | Original filesystem, TempHeap, App/database/localization/script providers, startup and owned cleanup pass |
| XP2 activation | Original Configure/Update and cleanup pass; one enabled/active mod and 46 components observed in getter order, not importer execution order |
| Rules data | Two XP2 diagnostic exports: 506 tables / 43,565 rows; exact Rust/Python file/schema/typed-row parity and refusal checks; separate base profile retained |
| Offline session | Original Attach/Leave and full cleanup pass for Base and XP2 contexts; session/local peer independently observed |
| Core benchmark Load | Native OK, all 76,556,938 input bytes consumed, successful Shutdown/session/framework/provider cleanup |
| Repeated Core Save | Four outputs across two independent processes are identical, 80,657,462 bytes each |
| Captured-output reload | Native OK and cleanup repeat; all 89,198 differing bytes classified as trade ordering or GameEffects identity/reference changes; native Save→Load is not byte-idempotent |
| Read-only loaded game facade | Both input profiles report turn 592, phase 0, local player 0 and false pending/update/processing flags; serialized before/after equality passes |
| Native actor/legality observation | All 36 actual player units visited; SkipTurn CanStart true for two; facade/readiness observations and Core Saves unchanged, cleanup passes; no request or action executed |
| Native cached dispatch | Query fails with controller initialized=false, GameCoreActive=false and null inner cache-player entry; original controller/cache initialization is the immediate prerequisite |
| Whole-save request preparation | Original RequestLoad retains an owned full-container copy and reset releases it; CheckRequest/Begin and later host finalization unverified |
| Observed serialized gameplay RNG | One complete 18-byte Game.m_kSyncRandom record is equal; all live RNG objects and omitted lock/lifecycle state remain unaudited |
| Typed trade classification/codec | All keyed fields in 179 origins / 1,571 edges match; wire iteration order differs; Rust exactly roundtrips both native graphs and every traced Route boundary, not trade gameplay |
| Rust primitives | Isolated Linux Random/normal record and stateless coordinate validity/distance match original code |
| Complete Rust state/actions/turns | Unsupported; no mutating native action or complete-turn differential comparison |

## Diagnostic profile and limits

The passing content profile gates eleven native modules and all 2,779 original content inputs. It uses an original constructor-state LoadGameViewState callback with request None and censorship=false, plus a genuinely constructed UI provider without graphics startup. Saved RequestLoad/Begin semantics are not inferred. Native getters report one enabled mod, one active mod and 46 components; their container order is not an execution trace. See [activation](linux-mod-activation.md), [rules input](rules-data.md) and [independent XP2 rules review](linux-xp2-rules-review.md).

Original session attachment precedes lower Core Load, supplying the genuinely observed local peer. This lower path is distinct from the complete host container/request workflow. Original CheckRequest/Begin, PostInitialize, active-user selection, pending settings and full saved-profile equivalence remain eventual full-host acceptance gates. The immediate selected-action prerequisite is narrower: genuine controller/cache initialization and its required original Uninitialize cleanup, rather than assuming that every UI stage must run first. [Save lifecycle](linux-save-lifecycle.md) and [action boundaries](linux-actions.md) retain the exact ordering/ownership contracts.

Benchmark-input Load and repeated original Saves reproduce across independent processes. Captured-output reloading also succeeds but produces a different repeatable Save. The [accepted normalization summary](../evidence/linux/save-normalization-summary.json) accounts for all 89,198 changed bytes without masking or planning-only exclusions:

- **89,156 Trade bytes:** all 179 origin memberships and 1,571 keyed Edge payloads match, including IDs, flags, yields and ordered paths. Wire record order differs. Native traversal is observable, so future callback, tie-breaking, RNG and gameplay equivalence remain unproven.
- **42 GameEffects bytes:** fourteen Team object IDs and 28 corresponding context/subject references in `DEFAULT_VICTORY_REQUIREMENT` follow the observed +24 ID mapping. Native Core Read includes victory-requirement reconstruction. The exact allocation/reuse/destruction sequence producing +24 and a global reference census remain unobserved; the accepted census covers the full RequirementSystem and fourteen Team suffix records only.

One observed complete 18-byte `Game::Instance::m_kSyncRandom` serialized record is equal between profiles. This is not all-live-RNG coverage: lock count is omitted and other instances/lifecycles still need observation. These bounded classifications do not establish complete state semantics. [Native owner/field evidence](linux-save-normalization.md), [Core Load/Save](linux-game-load.md) and [trade contracts](linux-trade-serialization-contracts.md) retain the exact limits.

Non-idempotent native Save→Load is observed target behavior to reproduce, not a defect to patch or a blanket blocker for action comparisons. Same-input fresh-process native baselines are repeatable and usable for scoped action differentials. Exact save import must reproduce native reconstruction; simulator cloning must separately preserve complete imported gameplay state.

The native read-only facade returns turn 592, phase 0 and Context/Game local player 0 in both profiles; pending messages, NeedsUpdate, NeedsTurnAdvance, IsAiProcessing and IsTurnProcessing are false. Wrapper release and these queries leave before/after Core Saves identical. These coarse observations do not imply action readiness, exact gameplay-state equivalence or complete in-scope live-state identity. Queues, caches, events, jobs and host/session state require separate coverage.

The read-only actor observer additionally queries genuine unit iterators and SkipTurn legality: 36 units, two accepted predicates, no requests or updates, identical before/after Core Saves and successful cleanup. The following cached-player query is a separate failed probe: controller and host Contexts match, but controller initialized and GameCore-active flags are false, and an inner cache player is null. The fault prevents a second Save and cleanup, so that failed query is not certified nonmutating. [Action evidence](linux-actions.md) identifies original Controller.Initialize/InitializeCache and required teardown as the next concrete bridge.

## Rust and tooling boundary

The new [Rust trade wire codec](linux-trade-codec.md) exactly roundtrips both actual 469,638-byte native graphs and all 1,571 Route boundaries in each. It preserves keyed payloads and observed wire order separately. This is serialization support, not trade mechanics: native ID-map callback order, omitted live finalizing state and complete gameplay effects remain unverified.

`c6-data` verifies registered base and activated-XP2 diagnostic SQLite profiles, retaining complete typed rows and metadata. SQLite row IDs are not established engine definition IDs; data loading and unit projections are not mechanics. Its timings measure data operations only. The `C6SN` envelope is simulator transport, not Civ VI save import; opaque retention cannot advance unknown state.

[Linux Random](linux-random.md) covers isolated arithmetic/state, independent objects and normal complete records; gameplay stream ownership/lifecycle, logging and legacy/failing streams remain excluded. [Linux geometry](linux-geometry.md) covers sentinel validity and stateless distance, not initialized maps, movement or pathfinding. Historical Windows evidence establishes no additional Linux coverage.

The private persistent `c6-linux.gpr` project has targeted decompiles, guarded MCP access and checksummed snapshots, including current canonical-stream contracts. This remains targeted analysis rather than completed whole-program/DWARF analysis; see [Ghidra tools](ghidra.md).

## Next concrete milestones

1. **Initialize the native action-delivery bridge.** Run original controller/cache initialization with its matching Uninitialize lifecycle, recheck cached player/active-turn readiness, and preserve the unchanged-state query controls before requesting any action. Keep this bounded prerequisite separate from eventual full-host Begin/finalization.
2. **Establish complete restorable gameplay state.** Observe the native +24 GameEffects allocation/reuse sequence where required, validate changed trade iteration effects, extend reference/RNG/live-state coverage and prove deterministic branching. Keep native reconstruction behavior, simulator cloning, raw byte equality and gameplay equivalence distinct; exclude planning-only fields only with proof.
3. **Execute a real legal action.** Use actual loaded units/players, complete legality predicates and synchronized host processing; compare all state, events and RNG effects before implementing matching Rust mechanics.
4. **Complete the saved profile and Rust systems.** Finish original saved Request/Begin, PostInitialize, active-user/cache and pending-setting semantics, then port complete gameplay state while preserving IDs, containers, iteration/event order, pending work and every random stream. Retain Linux-native fixtures per supported operation.
5. **Reproduce a complete externally controlled turn.** Match genuine completion messages, phases, player/control transitions and every gameplay subsystem; execute rules/upkeep and yield for each next player decision. Native AI policy execution cannot substitute for external decisions. Benchmark full workloads only once those paths exist.

The [README](../README.md) links active commands. [Native ABI review](linux-native-abi-review.md), [XP2 rules review](linux-xp2-rules-review.md), [runtime evidence](linux-runtime.md) and the owner documents above define what each checkpoint establishes. Full gameplay equivalence remains outstanding.
