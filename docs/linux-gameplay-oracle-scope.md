# Gameplay-only Linux oracle boundary

The target is an exact headless gameplay library with external policies for **all player decisions**. Full desktop/UI startup and full multiplayer packet admission are not completion requirements for that library. The intended oracle boundary is a trusted authoritative gameplay request, with exact rule validation, request-side semantics, ordered Core execution and gameplay-affecting callbacks, ending at the next external decision. Network/UI integration can be tested separately.

This is a scope and static contract, not a new gameplay implementation or native test. Rust has bounded primitives, wire/rules components and bounded state phases; complete gameplay state, action execution and turns remain unimplemented. The corrected [Core component](linux-core-component.md) provides reproducible original SkipTurn evidence, not general action/turn support. Its historical `host_admission_proven=false` and `full_action_acceptance=0` labels refer to that experiment's claims; they must not be interpreted as requiring desktop/network acceptance before a future exact **gameplay** action API can be supported.

The [scope evidence](../evidence/linux/gameplay-oracle-scope.json) pins 21 original methods and nine call edges for supplied Linux XP2 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. All discovery/acceptance here is Linux-only. Pinned cru121 reference attribution is in [reference.md](reference.md); raw new evidence remains local under `../c6-ghidra/evidence/linux-gameplay-oracle-scope/`, with no upload authorization.

## Two different contracts

| Contract | Inputs and obligations | Required for the gameplay library? |
|---|---|---|
| Authoritative gameplay | Explicit player/actor/operation/parameters from external policy; original rule predicates, request normalization and side effects; ordered messages; exact upkeep/callbacks/state/RNG; next genuine decision | Yes |
| Desktop/network ingress | Peer registration, packet routing/reliability/migration, host load-screen state, UI event production and user-interface lifecycle | No, unless explicitly testing that integration or a particular dependency actually supplies gameplay rules/state |

An explicit player identity does not authorize controlling another player's actors. Retain real owner/turn/phase/automation/pause and rules checks. A trusted API supplies decision authority directly instead of deriving it from a registered network peer; it does not bypass gameplay eligibility. If enabled modes have gameplay-relevant timers, cloud rules or external service results, their inputs/semantics still need an explicit supported contract. Their placement in the host does not exclude them.

The earlier [Core-ingress review](linux-core-oracle-boundary.md) correctly rejected direct ReceiveMessage as a substitute for native queue/worker semantics and rejected calling beneath a deferred host packet while claiming accepted **host Request** parity. Those are distinct from testing a fresh, trusted authoritative Core command. A gameplay-only oracle need not prove that the original host would accept a network packet while its UI load-state predicate is false/ambiguous. No deferred packet is consumed, rewritten or relabeled as accepted by this scope decision.

Native CanStart and real request/manager methods are the appropriate rule/normalization references, followed by original Core QueueMessage, deserialization, operation execution and mandatory continuation. They are not automatically a complete action contract: CanStart is a query, may invoke gameplay scripts, and is not complete legal-action enumeration. Public Cache RequestOperation itself checks cached owner activity and MakeRequest, rather than calling public CanStart. A legitimate composite request can prepend an unready message before operation validation in the resulting state. Do not invent a universal pre-state CanStart rejection that drops that native request ordering. The current active-player component can impose its narrower supported-state gate; a general action API must close the full ordering or reject unsupported states before mutation.

## Exact logical state, not native heap replication

The logical gameplay state comprises facts needed to reproduce legal choices, rules/upkeep, action consequences and future gameplay observations for the same external decisions and environment inputs. Native allocation addresses, pointer widths, vector capacity, unused padding and renderer object layout are not independently required equality targets. Preserve logical identities, ownership/aliasing, relevant collection contents, ordering, arithmetic and RNG behavior. If a native implementation detail demonstrably changes gameplay ordering or results, reproduce that effect; do not require the same numerical heap addresses.

| Native data | Scope test |
|---|---|
| Units, formations, players, maps, research, diplomacy/deals, notifications, NPC facts, rules configuration | Exact logical gameplay contents and transitions remain required |
| Memoized/path/query data | Its consumers and effects decide scope. A lazy getter that mutates gameplay, consumes RNG or changes future rule outcomes remains relevant; prove elimination/reconstruction or model the necessary logical contents |
| Presentation-only derived caches | May be omitted/rebuilt when ownership, consumers and effects establish that they only affect excluded presentation. Mere existence of a cache does not require exact native capacity/layout/addresses |
| Mixed reporting/cache callbacks | Separate the gameplay effects from display outputs; retain unclassified effects until proven. A namespace or event name is insufficient |
| Native AI planning | Excluded only for proven planning/choice policy. AI-owned diplomacy, gameplay facts, NPC state and gameplay RNG are not excluded by name |

The observed YieldMonitor is a concrete mixed dependency: active human count one, 6,996 plot entries and 182 trade entries cause original yield/trade checks, cache comparisons and possible recursive reporting. That does **not** by itself require reproducing its allocator addresses, capacity or native vector layout. It also does not prove it may be removed: lazy rules/path reads, logical cache contents and downstream callbacks are being classified separately. Unit snapshot getters and activity translators have the same consumer/effect test. See [reporting-provider evidence](linux-reporting-provider.md).

Unknown save bytes stay unclassified and unmasked while typed ownership is established. The raw Save→Load 89,198-byte failure, raw action stream differences and raw ASLR object differences remain reported. A proven physical pointer is compared through logical identity/aliasing when appropriate, with the raw observation retained; unknown bytes must not be swept into a pointer, UI or planning exclusion. Full Save equality is useful evidence but neither necessary for identical physical addresses nor sufficient for complete live-state equivalence.

## Required services and possible omission profiles

The original Context exports SetEngineInterface (`0x2c7c9f0`), forwarding a genuine IEngineUtility provider to Core globals. This is an actual service boundary, not evidence that every host service is rendering. The current oracle uses original providers and native ownership. Core imports, database/rules, localization, gameplay scripting, synchronization, request-side callbacks and reporting metadata remain required wherever exercised. Do not fabricate partial provider objects or vtables.

In particular:

- The rules/DLC/mod/configuration identity and original registration order are part of every supported fixture. Gameplay Lua can veto CanStart or alter state; missing callbacks are not equivalent to a script-bearing configuration.
- Cache registrations can execute synchronously during original Reporting Send. Preserve gameplay callbacks and any logical mirrors used by later legality/request queries. Do not label every cache update “UI.”
- Engine SendEvent performs translation/dirty bookkeeping and may call pre-publish subscribers before queueing presentation events. Proven display-only output can be outside the gameplay contract, but unclassified synchronous effects remain in scope.
- Locks/facade ownership, actual worker completion/parking and queued ordering are oracle implementation obligations. Native rendering loops, mouse input, audio output and display asset fidelity are excluded product behavior, rather than additional gameplay acceptance goals.

There are genuine native API leads, but no proved universal headless-equivalence switch in this research:

| Original API | Exact bounded finding | Interpretation |
|---|---|---|
| Context SetEngineInterface | Installs the provider through the actual Core API | A possible separately specified headless provider boundary; each gameplay-relevant service still needs exact semantics |
| Civ6App SetUIOff / IsUIOff | Setter exchanges one host flag; getter reads its inverse bit | Genuine presentation configuration lead. The inspected bodies do not unregister gameplay/reporting callbacks or prove a complete equivalent initialization profile |
| Civ6App SetupDedicatedServer | Inspected target body returns true without setup | Not proof that a usable dedicated/headless session is initialized |
| Reporting Event Enable / Context EnableCacheUpdates | Global reporting/cache dispatcher gates | Not UI-only filters. Disabling them can remove observed gameplay continuations and is not an exactness shortcut |
| Per-component registration lifecycle | YieldMonitor Initialize builds caches then RegisterEvents; its Uninitialize clears cache storage without a demonstrated selective unregister in that body | Not an authorized way to disable the observed receiver; lifecycle/consumer proof is required before proposing an absence profile |

A future native profile may legitimately avoid installing presentation consumers through their original initialization/subscription APIs, or use a fully specified provider that never creates a desktop renderer. That is a **prospective supported profile**, not permission to clear current callback tables or force flags. This investigation proves no particular registered consumer safe to remove end-to-end. The existing minimal original-provider component already demonstrates that complete desktop/UI execution is unnecessary for its bounded command; extending it requires closing gameplay dependencies, not recreating the desktop.

## Request-side TurnUnready is real ordering

Original UnitOperation GetUnreadiesTurn (`0x2d28e00`) returns true. Before serializing the operation, BroadcastMessage calls HandleSendTurnUnready (`0x2d24090`). It reads that virtual property, obtains the genuine local player, calls CanUnreadyTurn, and, if eligible, calls RequestTurnUnready with native subtype **APP_UI `0xd60b7e54`**. A successful request constructs and sends original PlayerTurnUnready through the same broadcast delegate **before** the original UnitOperation serialization/send. Broadcast continues the original operation even if Handle returns false. These effects cannot be dropped merely by replacing transport with a local Core queue.

Original Player CanUnreadyTurn (`0x2b31700`) is false immediately when the player is turn-active. Otherwise it requires simultaneous turns, no pending turn activation, and no game turn processing. RequestTurnUnready (`0x2b5bb00`) then checks the current phase's AllowTurnUnready rule and invokes engine utility slot +0x68. The original host callback checks its turn timer for APP_UI, changes controller sent-complete bookkeeping on allowed paths and can set UI SkipNextAutoEndTurn. The latter is UI automation policy excluded from the simulator's player decisions; phase/timer/turn rules or protocol state that affects gameplay cannot be discarded with that label. General support must separate those effects with an explicit provider/environment contract.

Queued PlayerTurnUnready Execute (`0x2d281c0`) rechecks eligibility, requests unready with subtype **MESSAGE_UNREADY `0x0a99df6c`**, then on success clears player turn-complete (NONE subtype), sets turn-active true and reports the local/remote unready event. Native setters/phase/provider continuation and ordered extra messages are part of an eligible composite request. Calling a setter alone is insufficient.

For the current corrected SkipTurn fixture, original readiness observes the real local player turn-active=true. The first branch therefore implies CanUnreadyTurn=false and excludes the entire prepend/provider path in that state. This is a static consequence of an observed native predicate, not a fabricated return. The **one concrete next observation** is a read-only original CanUnreadyTurn query on the same genuine local player at the quiescent pre-MakeRequest boundary, recording its expected false result with independent before/after state/RNG purity checks. No Broadcast, UI construction, packet admission or new action run is needed just to observe that predicate. Inactive/simultaneous/unready-eligible states remain unsupported by the current component until their complete native composite contract is observed.

## Gameplay acceptance criteria

An operation becomes an exact supported gameplay action only when all of these hold for its explicit profile:

1. The complete relevant logical initial state, original rules/configuration, gameplay RNG streams and external/environment inputs are identified and reproducible. Unknown ownership remains a gap, with raw differences retained.
2. Original legality, identity/owner/control/phase gates, parameter defaults, normalization and request-side ordering are preserved. Genuine rejected requests retain whatever effects the original actually produces; library Unsupported rejects before mutation.
3. Core command bytes, ordering, deserialization, ownership checks, manager execution, mandatory reporting/cache/script/diplomacy/notification/rules effects and synchronization match. A tuple of final Unit fields or undelivered report obligations is only a state phase.
4. Every actual player choice yields to external policy before native policy chooses or consumes policy-choice RNG; preserve the remaining rule continuation. All-human majors alone do not cover minors, barbarians, diplomacy, automation or off-turn jobs. A fatal guard after mutation is a failed diagnostic, never a resumable decision boundary.
5. Compare outputs, complete affected logical state, all gameplay RNG and relevant future-behavior/order observations using fresh controls, repeated branches and boundary/failure cases. Physical representation differences and excluded presentation may be classified only with concrete evidence.

No criterion requires full host network admission or desktop lifecycle parity. No criterion permits deleting a gameplay dependency because it happens to live in the host, cache, scripting or AI namespace. `Engine::apply` remains Unsupported until its actual gameplay obligations close; UI integration is a separate task rather than a hidden replacement objective.

Reproduce the static facts, without original-code execution:

```sh
python3 tools/linux/gameplay_oracle_scope_audit.py --output target/linux/gameplay-oracle-scope.json
```
