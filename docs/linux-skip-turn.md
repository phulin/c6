# Linux SKIP_TURN mutation contract

The library still implements isolated primitives. This is a **bounded static contract**, not an implemented Rust action, native execution result, complete affected-state model or state/action/turn equivalence. The original Linux XP2 SHA-256 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. A Rust activity toggle cannot be accepted as SKIP_TURN.

Reproduce the exact-byte, call and relocated-slot checks with:

```sh
python3 tools/linux/skip_turn_audit.py --output target/linux/skip-turn-contracts.json
```

The [checked report](../evidence/linux/skip-turn-contracts.json) records the verified target, method fingerprints and explicit gaps. Discovery consulted cru121's pinned revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, CC BY 4.0, under the [reference attribution](reference.md). Conclusions below were checked against the supplied target's symbols, instructions, ELF relocations and matching DWARF. Raw disassembly, GDB type layouts and Ghidra output remain private in `../c6-ghidra/evidence/linux-skip-turn/`. Decompiled `Clear`, `DeleteNode` and notification tails contain warnings; original instructions establish their tail calls. One secondary `GetEndTurnBlocking` decompilation failed; its actual instructions were inspected instead. No decompiler success is counted as native acceptance.

## Legality is layered

The exported predicate uses the original operations manager, as described in [the action boundary](linux-actions.md). `Manager::CanStart` at `0x2da20f0` rejects a busy unit. With `testVisible=false`, it also checks tactical commands for the owner, active turn or matching automation-processing player, and pause state for a human. A nonempty queue requires that the current operation permit interruption by the requested operation. Disabled operations reject. The actual script system, when present, runs `CallTestAll("CanStartOperation", owner, unitID, numericOperationID)`; a successful callback invocation with false result vetoes. A real target/current plot is required, then the registered handler predicate runs. `testVisible=true` bypasses specific manager gates; it does not bypass the SkipTurn handler's human checks.

`SkipTurn::CanStart` at `0x2dac950` has this precise leaf predicate:

1. For a human unit, reject `GetFortifyTurns() > 0`, activity HOLD (`0x047c048b`) or SLEEP (`0x024a74eb`). At the unit's actual layered location, obtain the actual map unit collection and count friendly units of its formation type with the helper's bool argument false. Reject when that count is **unsigned greater than** `PLOT_UNIT_LIMIT`; equality passes. Formation membership, placement-helper semantics and the rules parameter must remain exact inputs.
2. For every unit, reject activity INTERCEPT (`0x5ff1bbdf`). Require `unit.GetEspionage().GetOperation() == -1`.

The handler ignores the operation, target plot, visible flag and results map after receipt. Its leaf does not check movement remaining. Neither this leaf nor the manager predicate alone establishes cached request eligibility, valid session delivery or successful execution. Scripts can have effects even during a predicate call; do not generalize the benchmark's unchanged Save bytes to every callback configuration.

## Request and manager execution

The operation hash is `0x2c681908`; the real registry resolves it to a numeric handler ID. The hash must not replace that ID in queue nodes or messages. `Context::RequestUnitOperation` delegates to the cache manager, which checks owner range and the actual cached owner's active turn, constructs the original Request, invokes manager conversion and broadcasts an original UnitOperation with `InsertMode::USE_REQUEST = 0`. It does not directly mutate the live unit.

DWARF identifies Request `bAppend` at `+0x1c`, `bFromUser` at `+0x1d`; both constructors set both false. The inherited parameter conversion reads flags at `+0x18`, a modifier at `+4`, and optional shared parameters, but does not set those two booleans. Therefore the observed public empty-map path starts with both false. Do not assume that an external request is marked `bFromUser` just because the caller is human. Other paths/parameters require their own original request capture.

`UnitOperation::Execute` at `0x2d28a00` resolves the owner through the real player manager and finds the actual unit. Invalid player or missing unit returns immediately. Otherwise it invokes manager `Add(unit, request, insertMode)`, then **always** processes pending diplomacy and updates that owner's end-turn blocking state. A rejected operation inside `Add` does not skip those final calls. The message returns void.

`Manager::Add` at `0x2da1a20` performs the following work, in order:

- Construct a node. Clear the old queue when mode is CLEAR (`1`), or mode USE_REQUEST (`0`) with `bAppend=false`; remember whether this branch ran. If `bFromUser`, call `SetAutomateType(unit, NO_AUTOMATE=-1)`. That helper can clear the queue and set AWAKE when automation changes. The public empty-map request does not take this flag branch.
- Resolve the handler; if absent, stop after any earlier clear/automation effects. Copy the request and run handler `CheckInsert`. SkipTurn uses the inherited true-returning handler.
- Copy operation data into the node. Set `iPushTurn` from the current game turn and capture current activity/automation in `kPreOperationState`. Insert at the queue end through tracked `m_kOperations.edit()`, emit event `0x15c35863` with unit identity and numeric operation ID, and activate only if resulting queue size is one and the earlier clear branch ran.
- After insertion/activation returns, if the copied request has `bFromUser` and owner equals local player, emit `0x7b4264e9`. This is conditional and follows immediate operation execution.

Clear deactivates, repeatedly pops the original queue nodes, then clears the actual formation path cache when a formation exists. Those old handlers may have gameplay effects. A Rust first action may explicitly reject unmodeled queue contents without mutation, but must not claim native parity for those cases.

## Immediate SkipTurn effect sequence

`Activate` at `0x2da2790` first returns for an empty queue or busy unit. Existing activity OPERATION (`0x490cc8e1`) enters the existing operation's `Continue` path. Otherwise it calls full manager legality again with the real front node/current plot and false/null query defaults. Legal activation constructs `StartData { bDelete:false, bAction:false }` and invokes manager `Start`.

Manager `Start` at `0x2da2340` sets activity to OPERATION when `unit.CanMove()` is true, otherwise HOLD, **before** calling the handler. It then retrieves the real queue front and handler. `SkipTurn::Start` at `0x2dac8e0` emits reporting event `0x3d4006f2` with the registered operation hash as event subtype, actual unit identity and integer zero; calls manager `SetActivityType(HOLD)`; and writes `StartData.bDelete=true`. It leaves `bAction=false`. Treating this field as a generic completion state loses the actual queue deletion contract.

`SetActivityType` at `0x2da1840` returns if the value is already equal. Otherwise it makes the original tracked assignment, registering the variable in its owning archive's dirty-variable tree when an archive exists, then emits event `0xe23b112d` with the unit identity and new activity. A movable legal unit can therefore emit OPERATION then HOLD activity changes; comparing only final HOLD misses this behavior. No movement-point decrement or fortification increment occurs directly in this leaf. Transitive effects still require observation.

With `bDelete=true`, Activate calls `PopFront`. SkipTurn inherits `BaseHandler::Deactivate` (`0x2d964c0`), which emits `0xa60328c5` with unit identity, registered operation hash and `IsReadyToSelect(unit)`, then returns false. That readiness helper requires local ownership, no automation, CanMove, queue size unsigned at most one, and activity OPERATION or AWAKE. The ordinary final HOLD branch therefore reports false. False causes `DeleteNode`; deleting the front deactivates the manager, zeroes its tracked operation timer, removes the node and clears the formation path cache if the queue becomes empty. Manager deactivation changes OPERATION to AWAKE only; HOLD survives. `PopFront` emits `0x383669f1` when the resulting queue is empty. Activate may continue with further queued nodes unless delayed death or empty queue stops it. Event subscribers can change the path, so this is an instruction-derived sequence under unchanged subscriber inputs, not an observed event transcript.

## Fields and observation surface

These are exact Linux offsets from the original object, not portable simulator serialization layouts:

| Object / field | Archive wrapper | Value / size |
|---|---|---|
| Unit CInstance `m_eAutomation` | `+0x268` | `+0x278`, enum32 |
| Unit CInstance `m_eActivity` | `+0x280` | `+0x290`, enum32 |
| Unit Instance `m_iOperationTimer` | `+0x518` | `+0x528`, int32 |
| Unit Instance `m_kOperations` | `+0xc00` | `+0xc10`, 200-byte OperationQueue |
| Node `iPushTurn` | none | `+0x1c`, int32 |
| Node pre-operation activity / automation | none | `+0x20` / `+0x24`, enum32 each |
| StartData `bDelete` / `bAction` | none | `+0` / `+1`, bool each |
| Player end-turn blocking setter store | none in this setter | `+0x7a8`, enum32 |

Queue values are original typed nodes with shared parameter ownership, original ordering and tracked-edit side effects. The 200 bytes are not a sufficient Rust state model. Observe the queue through its original API and record every node's operation, flags, parameters, push turn and saved pre-operation state; observe archive dirtiness separately. Preserve formation path/cache changes and reporting/cache subscribers as explicit requirements rather than masking their bytes. Typed observer offsets require the same module gate and real object provenance; they do not authorize synthetic objects.

## Diplomacy, notification and RNG boundary

`Diplomacy::Manager::ProcessPending` (`0x2913510`) copies the real ever-alive player vector and visits it in that order. For each `IsAI()` player it repeatedly calls `ProcessPendingSessionStatement(player,-1,direction=1)` until false. That method (`0x2911900`) walks original open sessions in linked-list order, matching player to either endpoint (and an optional other-player constraint), removes the next matching pending statement, calls `ProcessStatement` (`0x29119b0`) and returns true; no match returns false. Thus it drains queued statements, but **the statement processing may create further decisions/effects**. Empty open sessions prove this particular path calls no statement processor; a generic no-op diplomacy stub is invalid. Exact response/policy branches, statement effects and RNG remain a required transitive contract; see [policy boundary research](linux-policy-boundary.md).

`UpdateEndTurnBlockingState` (`0x2b5bd70`) obtains notification blocking state. If the player's real global turn-complete flag is false, it invokes notification `Update` then reads blocking state again. Update sends auto notifications, conditionally processes auto-activation (PlayByCloud gates apply), dismisses expired notifications and cleans up. These dispatch to actual notification subclasses and can report/activate effects. Finally `SetEndTurnBlockingState` (`0x2b5be10`) compares/stores the player's value at `+0x7a8`; on a change for the local player it emits `0x55f15a7b` with old/new values. Snapshot notifications and callback effects; the player enum alone is incomplete.

There is no direct RNG call in the audited SkipTurn leaf. This does **not** prove action RNG neutrality: script vetoes, reporting subscribers, preexisting queue handlers, formation work, diplomacy and notification subtype callbacks remain reachable. Record all complete gameplay RNG objects before/after; native AI-namespaced gameplay effects remain in scope. Every player decision must yield to external policy. A guard may refuse an unmodeled callback/decision path before mutation; it cannot silently replace policy or force a turn boundary.

## Required native cases and Rust entry point

A first native fixture must use the genuinely delivered public request and synchronized host drain owned by the action-boundary work. Starting from the same identified original fixture, capture full Core Saves, the raw byte diff, all gameplay RNG, ordered request/message/reporting events, actual unit/formation fields and queue, archive dirtiness, cache/event stream changes, all open diplomacy sessions/pending statements and player notification/end-turn state. Reproduce both branches from the same full original input. Keep the recorded Save→Load **89,198-byte failure** visible while typed ownership is investigated; do not mask unknown bytes. No native mutation was run for this contract, and no runner lease was taken.

The minimal boundary matrix must cover: busy; phase/player inactive; paused human; script absent/allow/veto; disabled operation; absent plot; empty versus interruptible/uninterruptible queue; human/nonhuman; fortify turns zero/positive; AWAKE/HOLD/SLEEP/INTERCEPT/OPERATION; count below/equal/above the real plot limit; espionage `-1` versus active; zero versus positive CanMove; automation unchanged/changed; empty versus pending diplomacy; complete versus incomplete owner turn; notifications unchanged/changed. Reject unsupported setup explicitly and preserve minimized differing cases.

A safe first Rust surface is a Linux-profile typed legality/transition module behind an explicit supported-state gate, with operation identity, complete modeled unit/formation/queue state and ordered effect output. Implement the verified handler predicate as a separately scoped helper only after exact placement/count inputs exist. An accepted action must compose manager request conversion, queue/dirty-state/event behavior, diplomacy and notification effects, or reject those unmodeled states **without mutation**. Keep this entry point unsupported until the native first-action fixture, reversible input restoration, complete affected-state observation and differential comparisons exist. A helper returning HOLD, a final-field snapshot or successful static audit is not that acceptance.
