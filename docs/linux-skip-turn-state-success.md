# Quiescent conditional SkipTurn state footprint

The first completed conditional Core SkipTurn changes one Unit, its Formation and owner 0's notification list. Those three records account for the entire captured serialized difference. A fresh independent process repeats the same before/after streams and typed effects. This is a bounded native component result, not full-host admission, a Rust transition implementation or complete gameplay state/action/turn parity.

The [state audit](../evidence/linux/skip-turn-state-execute-3.json) binds the exact Linux XP2/host gates, configuration `d68604a55344b85212154f5abf20b2a109e2d65b34a582686226e1f3f520187a`, actual selected owner 0/unit 15794187, command delivery, source identities and original serializer evidence. Execution 3 and fresh repeat 4 each have one Queue/Receive/UnitOperation execution and successful native cleanup. Original execution-counter completion, Join, Synchronize and worker suspension precede observations/Saves. Earlier executions 1/2 remain separate failures; their matching captures do not supply coherent poststates.

The before stream is 80,657,462 bytes, SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`. Both completed after streams and repeated-after captures are 80,656,254 bytes, SHA-256 `be08f44a4b83bd7eaa8d15a24443731764705c32961c570697bc3bb1736097bc`. Raw same-offset comparison retains **46,037,717 differing common bytes plus 1,208 bytes of length difference**, total 46,038,925; nothing is masked. The large raw count includes later byte shifts and remains reported alongside the structural explanation.

## Exact affected records

| Original serializer owner | Before half-open range | After half-open range | Size change |
|---|---|---|---:|
| Formation `(0,16842764)` | `[4697095,4697251)` | `[4697095,4697151)` | -100 |
| Unit `(0,15794187)` | `[5052651,5057469)` | `[5052551,5057325)` | -44 |
| PlayerNotifications, owner 0 | `[76400879,76404572)` | `[76400735,76403364)` | -1,064 |

All bytes before, between and after these three records match exactly at their aligned offsets. This includes the Trade graph, RequirementSystem, other Units/Formations and the complete remaining stream. Requirements/Trade have **no serialized change** in this fixture. This does not establish complete live-state equality for those systems or classify any AI-owned state as planning-only.

Original before-owner boundaries come from the pinned serializer trace, not ID-byte searches. After Formation boundaries follow its count/path/member framing and production codec roundtrip. The Unit's candidate extent follows its original queue field's 52-byte MoveTo record becoming an 8-byte empty queue; complete production decoding, exact roundtrip and all 73 field comparisons verify that extent. Notification count/record framing independently consumes its complete after region. Exact equality of all intervening bytes and the suffix checks the complete decomposition, rather than assuming every later offset shifts by a fixed amount.

Only Unit `m_eActivity` and `m_kOperations` differ; the other **71 serialized fields** match exactly. Activity changes OPERATION `0x490cc8e1` to HOLD `0x047c048b`. The existing MoveTo node disappears, leaving an empty queue. Automation stays -1, operation timer stays 0, and raw movement stays 256. The original queue's typed parameters, directive, flags, saved state and push turn are recorded in the report and [initial-state contract](linux-skip-turn-state.md). These are observed fixture values, not hard-coded selection rules.

Unit dirty-index history grows from 32 to 35 entries, adding **11, 32 and 72** in the original returned order. Timer assignment therefore matters even though its final serialized value remains zero. Both identified RNG objects retain all 32 raw bytes within each process, including seeds/counters; cross-process address-containing differences are separately retained without masking. Complete gameplay RNG remains unresolved.

The Formation's five ordered path nodes are removed, its cache location changes `(85,58)` to `(-9999,-9999)`, and its member list remains `[15794187]`. All other serialized Formation fields match. Formation dirty history and omitted live evaluating-operation state were not observed; do not infer their complete equality from this record.

## Notification upkeep is part of command aftermath

The owner 0 PlayerNotifications header remains version 1, owner 0, next ID 49. Count changes **12 to 9**, preserving surviving order. Cleanup removes three records whose `IsDismissed` flags were already true in the before state:

| Notification ID | Type hash | Before byte length | Location |
|---:|---|---:|---|
| 23 | `0xe90d955a` | 354 | `(57,27)` |
| 42 | `0xe90d955a` | 359 | `(76,26)` |
| 44 | `0xe90d955a` | 351 | `(14,16)` |

Their lengths sum to **1,064 bytes**. They are storm notifications; their deletion is general dismissed-list cleanup, not a SkipTurn-specific rule or evidence of a new storm outcome.

Surviving notification **47**, type hash `0x0c19b649`, changes only `HasBeenActivated` from false to true. Its two moment parameters remain unchanged: `MomentTypeHash=3535159053`, `MomentID=10438`. Every other byte of every surviving notification matches. Original getter `0x2d313c0` reads native bool `+0x19e`; original Activate at `0x2d2dd80` sets it at `0x2d2dd8e`.

Activate also calls **`Event::Send(Type, Value const&, Value const&)`**, event `0x025f6cfd`, at `0x2d2dde4`. Values include notification identity under key `0xeab69e5a` and the actual activation bool under key `0x78a2f946`. This overload is **unobserved by the existing identity-event hooks**. The nine recorded Unit events are consequently an incomplete reporting inventory. The native autoactivation loop supplies false to the virtual Activate call; the general event payload itself still needs native capture. Its subscribers and complete effects remain in scope; no presentation-only classification is justified.

The nine observed identity events preserve this order: manager changes old OPERATION activity to hard-coded AWAKE; old MoveTo deactivation with ready=true; old empty queue; new SkipTurn insertion; OPERATION activity; SkipTurn start; HOLD activity; SkipTurn deactivation with ready=false; new empty queue. The saved old-node activity also happens to be AWAKE, but Manager::Deactivate does not read it to restore activity. The old queue's cancellation and the later notification update make a leaf-only HOLD toggle insufficient.

Executions 3/4 also retain a historical observer ABI limitation: their three identity-event wrappers were declared void although original Event::Send returns int. This audit classifies actual captured bytes and records the five native call sites for the nine events; BaseHandler deactivation overwrites EAX with false, insertion/SkipTurn overwrite RAX through subsequent original calls, and activity/PopFront end in void epilogues. This does not silently certify the wrapper ABI or complete observer purity. The event observer owner is correcting return forwarding and arranging separately identified validation; the historical source identities remain frozen.

## General rules required by the next transition

Original UnitOperation calls pending diplomacy processing and owner UpdateEndTurnBlocking after the unit manager. The latter (`0x2b5bd70`) reads notification blocking state; when the actual global owner turn-complete flag is false, it runs Notification::Manager::Update and reads blocking state again, then sets the owner's blocking value. Matching final Player bytes does not eliminate this intermediate work.

Original notification Update (`0x2d2f360`) runs **SendAutoNotify → conditional ProcessAutoActivate → DismissExpired → Cleanup**, in that order. Autoactivation is permitted when not PlayByCloud, or when the manager's real game pointer exists and its local player equals the requested owner. The remaining passes still run when this cloud gate excludes activation.

The next implementation must use these inputs and predicates rather than the captured IDs:

1. `AcceptsNotifications` (`0x2d2f000`) requires an initialized manager (nonempty player-notification container vector), a game that is not uninitializing, and actual human player configuration. SendAutoNotify and ProcessAutoActivate use this predicate.
2. SendAutoNotify (`0x2d2f4b0`) visits real notification definitions in native order, testing `IsAutoNotify`. It constructs through the registered factory, releases redundant results, and calls the actual notification send/Add path for nonredundant results. Definitions, factory subtype behavior and duplicate/subscriber effects require exact support or explicit rejection.
3. ProcessAutoActivate (`0x2d2f5a0`) also requires the real player to be turn-active. Its severity threshold is signed enum value 0 for non-network games and 3 for network games. It repeatedly calls FindAutoActivate, stops on null or a locked result, otherwise calls that instance's actual virtual Activate(false) and rescans.
4. FindAutoActivate (`0x2d30ec0`) scans list order. A nonnull record must satisfy IsAutoActivate and either meet the signed severity threshold or have true variant key `0xc362976f`. An eligible **locked** record returns immediately, even after an earlier unlocked candidate. Otherwise retain the first eligible record whose HasBeenActivated is false, continue scanning for a locked blocker, and return the retained candidate at the end. This method has no direct dismissed-record filter.
5. IsAutoActivate (`0x2d2e1f0`) returns true when already activated. Otherwise, a non-network game with a found definition and `IsShowIconSinglePlayer=false` returns true; remaining cases use native bool `+0x19d`. FindAutoActivate's separate HasBeenActivated test must not be collapsed into this predicate.
6. IsLocked (`0x2ab70e0`) tests **signed live int `+0x194 > 0`**. That lock count is **not serialized** by Instance::Write. The v5 wire record alone cannot execute exact selection. Definition flags, configuration gates, manager/game state and subtype virtual methods are also required live inputs.
7. DismissExpired (`0x2d31130`) skips already-dismissed records; others pass their actual virtual expiration predicate before virtual Dismiss. Cleanup (`0x2d311e0`) visits list order and calls virtual Release then erases every IsDismissed node, regardless of its ID/type. The observed removed records were dismissed beforehand; no new expiration is inferred.

The static method identities and 39 inherited notification writer slots are recorded in the report. Raw original instructions are private under `evidence/linux-skip-turn-state/execute-3-contracts/`; these new raw files have not been committed or pushed. The notification codec owner has the frozen schema and observed slices. End-turn continuation, subtype effects, live lock counts, complete event delivery/subscribers, formation dirty state and complete RNG still block an accepted full Rust action.

The separate [generic aftermath contract](../evidence/linux/skip-turn-state-aftermath.json) checks 39 exact original branch sites and all eight relevant virtual bindings for both base and XP2 notification managers. It names the required live/rules inputs and specifies selection/cleanup without fixture IDs. Reproduce with `python3 tools/linux/skip_turn_state_aftermath_audit.py --output target/linux/skip-turn-state-aftermath.json`. This is static evidence; no native boundary matrix or Rust operation is supplied.

That contract also retains a second conditional reporting gap: changed owner blocking state `+0x7a8` sends `Event::Send(Type,int,int)`, event `0x55f15a7b`, with old/new enums at `0x2b5be54`. The existing nine identity-event hooks cannot establish whether this overload ran. The selected live observer does not read the blocking field, so matching Player stream bytes alone must not substitute for its complete live contract.

## Unit/Formation kernel handoff

The concrete MoveTo deactivation slot is `0x3affa30 + 16 + 0x68 → 0x2da4b40`. It first calls Unit::Placement::CompleteMovement (`0x2c42ab0`), then tail-calls BaseHandler::Deactivate. CompleteMovement returns immediately when `m_bNeedsMovementCompletion` is false. Otherwise it tracks that field to false (Unit index 50, wrapper `+0x6c8`, value `+0x6d8`) and emits identity/PlotCoord event `0x6e7ef927`. The actual before record contains false and the after record agrees. A bounded kernel may reject the true input before mutation while this additional event path remains unsupported; it must still recognize the concrete override.

Clear (`0x2da2710`) returns immediately for an initially empty queue, without timer assignment or Formation clear. For a nonempty queue it calls Manager::Deactivate (`0x2da29c0`), which changes OPERATION to the constant AWAKE and then assigns timer zero; other activity values survive, but nonempty-queue timer assignment still occurs. Clear repeatedly calls PopFront until empty, then clears the Formation path if a Formation exists. DeleteNode (`0x2da2680`) first marks the queue edited, deactivates when deleting the front, removes the node, and clears the Formation path if the result is empty. The instruction-derived one-MoveTo replacement branch, with unchanged subscriber inputs, invokes path clear three times: old final-node deletion, Clear's tail, new SkipTurn final-node deletion. Native invocation counts were not captured. Repeated dirty inserts deduplicate wrapper pointers without removing the tracked mutations.

Formation::ClearPathCache (`0x293d8e0`) performs tracked path edit/clear (index 14, wrapper `+0x1f0`) followed by tracked cache edit/SetInvalid (index 15, wrapper `+0x218`), even when already empty/invalid. It does not directly write evaluating-operation. That omitted live bool has its own tracked setter (`0x2da30d0`), index 13, wrapper `+0x1d8`, value `+0x1e8`; no live before/after observation is supplied for it. MoveTo's Unit CompleteMovement and the inspected manager Start body also contain no direct assignment to this Formation field. Reporting/script/subscriber effects remain separate dependencies.

The manager insertion/SkipTurn sequence must retain the original clear-before-request-copy order, capture current activity/automation and current push turn, mark queue insertion, run full activation legality, set OPERATION or HOLD according to the actual CanMove result, run SkipTurn Start and delete its node through PopFront. SkipTurn leaves StartData action=false, so the manager's action-dependent post-start wait branch is skipped. Busy, continuation, delayed-death, script, unsupported old-handler and callback environments need explicit gates rather than invented behavior.

## Reporting dispatch boundary

The common original map sender (`0x2d4a4b0`) checks reporting enabled/not suspended, calls Cache::Dispatcher::Send (`0x2723550`), then invokes real EngineUtility virtual `+0xa8`. Cache dispatch, when updates are enabled, looks up the matching handler vector and **synchronously invokes each nonempty delegate** at `0x2723619`. It is not itself merely an append-only output queue. The actual handler targets and current enable flag have not been captured in these fixtures.

The pinned host's real EngineUtility slot resolves to SendEvent (`0x32b4260`). It performs parameter-usage-based dirty-component bookkeeping and calls Translate::Dispatch (`0x3214c50`), whose registered translator callback is invoked synchronously when present. The original registration code maps relevant hashes to host UnitActivityChanged/operation and NotificationActivated translators. Representative GameCoreEvent Publish methods for operation-cleared event 71 (`0x32176b0`) and notification-activated event 257 (`0x321cac0`) first invoke an optional **synchronous pre-publish subscriber**, then create a queued host event and unlock it.

Both synchronous dispatch surfaces and a later queued-event branch are therefore proven. Their actual current callback registrations, receiver state and transitive effects remain unobserved. This evidence proves neither pure queued reporting nor a specific synchronous gameplay mutation in the current profile. A kernel reporting environment must bind those concrete gates; it should not manufacture generic continuations or exclude caches/subscribers by namespace. The separate static aftermath report now fingerprints this handoff and the real host provider as well as the notification rules.

## Reproduction and validation

```sh
python3 tools/linux/skip_turn_state_success.py \
  --runtime-report /path/to/closed-native-execute-3/runtime.json \
  --repeat-runtime-report /path/to/closed-native-execute-4/runtime.json \
  --output target/linux/skip-turn-state-execute-3.json
```

This runs the completed component artifact checker and production Rust wire codecs; it executes no original code. Both selected Unit/Formation pairs roundtrip exactly; all 21 notification instances consume their bounded regions, and each typed parameter map roundtrips exactly. The notification adapter explicitly supports only observed v1 containers, v5 base instances and current zero-argument localized strings; unsupported forms reject. Complete stream gap/suffix equality, all 73 Unit fields, known Formation framing and ordered notification deletion/activation checks pass. The independent fresh repeat has the identical serialized footprint and typed observations; raw address-containing differences remain reported.

Unsupported Formation versions, wrong original module hashes and the failed legacy execution reject. Python compilation and whitespace checks pass. The Rust gameplay library is unchanged by this research checkpoint.

No new native run or after-serializer trace was needed. The historical **89,198-byte raw Save→Load failure** remains unclassified and unmasked. This successful conditional fixture does not repair branch restoration, prove universal external-policy boundaries, authorize native AI decisions or establish full-host/Rust gameplay acceptance. Original-code attribution remains in [reference.md](reference.md).
