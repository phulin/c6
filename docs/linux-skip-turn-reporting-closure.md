# Conditional SkipTurn synchronous reporting closure

The remaining host dirty-component provider effect now has a concrete typed contract and a bounded Rust implementation. It is an ordered fanout into registered ID vectors, not an arbitrary subscriber invocation. Actual dirty registrations and vector contents remain uncaptured. The existing eleven balanced Send/return calls therefore still do not admit a complete Rust SkipTurn, gameplay state, actions or turns.

The [audit](../tools/linux/skip_turn_reporting_closure_audit.py) and [result](../evidence/linux/skip-turn-reporting-closure.json) check the pinned Linux XP2 and host hashes, 23 complete method bodies and five direct call sites. They reuse completed original captures; no original code, engine, debugger or new observer runs here. The [Rust boundary](../crates/c6-core/src/reporting_dirty_components.rs) implements only already-decoded dirty fanout with explicit receiver identities and registrations. It has source-derived tests, **not accepted native differential parity**.

## Actual call inventory

The accepted provider replay's armed phase resolves these synchronous surfaces. The queue-complete call has no registered translator hit; absence of a hit is not a universal missing-registration census. The original typed senders bypass the generic map/vector sender checkpoints, as corrected in the existing provider report.

| Send event | Calls | Observed cache callback | Observed translator / host Publish |
|---|---:|---|---|
| UnitActivityChanged `e23b112d` | 3 | Unit snapshot each time | `320b670` / 35 |
| UnitOperationDeactivated `a60328c5` | 2 | None captured | `320ccb0` / 70 |
| UnitOperationsCleared `383669f1` | 2 | Unit snapshot each time | `320cd30` / 71 |
| UnitOperationAdded `15c35863` | 1 | Unit snapshot | `320cb30` / 67 |
| UnitOperationStarted `3d4006f2` | 1 | Unit snapshot | `320cbb0` / 68 |
| NotificationActivated `025f6cfd` | 1 | None captured | `3212d00` / 257 |
| Queue complete `9ea4c222` | 1 | YieldMonitor `2c66430` | No translator hit captured |

Seven Unit snapshots additionally Publish host CacheUpdated 275. All **17 actual pre-publish getter results are null**: event275 seven times, event35 three, events70/71 twice, events67/68/257 once. Those synchronous optional callbacks are closed as absent for these calls. They must not be reintroduced as an unknown exercised subscriber, or extended to queued-event consumers or other profiles. Ten actual translator pointers and eight cache targets are recorded with their original sequence numbers in the provider parent.

The activity translator's two actual facade virtual bindings and parameter-free SkipTurn writer are already evidenced by the provider audit. This checkpoint does not reopen those bodies. Unit snapshot transitive getter state, monitor numerical plot/trade rules and later queued-event consumers retain their separately documented limits. The successful passive monitor interval establishes endpoint equality only on its captured cache/Trade/PlotYields surfaces. It does not replace numerical rule implementation or cover allocator/object/RNG state.

## Newly closed provider dependency

`EngineUtility::SendEvent` first calls `GetEventCompoments` at host `32b427a`. That lookup (`32b6200`) finds an existing event-map node and returns its ComponentsArray at pair+8, or null. It does not insert a registration. While iterating original typed-map parameters, the sender decodes known identity keys into stack EventData. A recognized identity marks dirty only when the registration is nonnull and its actual ParameterUsageDefinition has bit zero set (`ChangesObject`, `32b5620`). The seven identity branches converge on the single SetDirty call at `32b4b97`, before Translate::Dispatch at `32b4c03`. Other decoded scalar fields do not independently imply dirty fanout.

`ComponentsArray::SetDirty` (`32b5f90`, full body 66 bytes) scans the actual receiver-pointer vector in native order. Each pointer is passed directly to concrete `DirtyComponents::Add` (`32b5950`, 86 bytes). There is **no receiver vtable or delegate dispatch** in either method. An unknown registration therefore no longer means an unknown synchronous function target on this surface.

Add scans that receiver's existing ComponentIDs in order, returning immediately on equality; otherwise it appends a fieldwise copy. ComponentID is exactly eight bytes: owner u16 at +0, type u16 at +2, ID u32 at +4. Native equality (`2fb7350`) compares all three fields; native copy (`2feaef0`) copies all three. Type is part of identity even when owner and numeric ID match. Existing order is retained, a repeated identity does not move, and each receiver's list deduplicates independently. This is distinct from the Core Unit tracked-variable dirty-index history.

The direct writes are to the receiver ID vector and its storage. A full vector takes the native EASTL allocation/copy/free path; an append with spare capacity copies eight bytes and advances end. The audit pins those original helper bodies, including capacity growth and empty element destruction. It does not claim allocator state, failures, replacement allocator effects or every transitive allocation helper is inert. No gameplay object setter, RNG call or reporting callback occurs in the inspected SetDirty/Add/equality/copy bodies. These are scoped source facts, not an assertion that the whole SendEvent call or all later consumers are pure.

The Rust function accepts an explicit registration state: unknown, absent, or an ordered receiver-index list. Unknown and invalid receiver inputs reject before any mutation. ChangesObject=false skips fanout. Valid fanout preserves ordered identity dedup. It does not invent actual registrations, decode event maps, fetch usage definitions, emulate native capacity/address allocation, run translators, drain queued events or advance gameplay. Native registration APIs normally deduplicate receiver pointers; a repeated supplied pointer still produces the source-consistent repeated Add calls and one stored identity.

## Precise remaining capture need

No new native replay is requested or authorized by this checkpoint. The current handler owner's frozen profile stays unchanged. For a separately scheduled future provider profile, three read-only hardware sites suffice to supply the missing facts without executing methods from the debugger:

1. At `32b427f`, retain event R15D and actual lookup RAX, including null. If nonnull, preserve raw32 vector header and ordered receiver pointers.
2. At `32b4b97`, preserve actual registration RDI, raw8 ComponentID at RSI, each receiver raw32 vector header and complete used ID bytes. This identifies precisely which parameter passed the original ChangesObject gate.
3. At `32b4c08`, preserve those same receiver headers/used bytes after fanout and translator. Correlate thread, original stack and balanced Send interval; preserve every interval, including no-SetDirty paths.

The actual registration and receiver snapshots can then drive the bounded Rust implementation and compare all affected logical IDs, while retaining raw addresses/capacity differences separately. Original command, all five Save identities, worker quiescence, cleanup and deadline remain required. No inferior calls, field writes, callback substitutions or relaxed flag gates are needed. Receiver/index binding and any data-budget failure must be explicit. This is a precise missing original-fact specification, not another unchanged observer or a claim that registrations are empty.

Reproduce the offline audit with `python3 tools/linux/skip_turn_reporting_closure_audit.py --output target/linux/skip-turn-reporting-closure.json`. The isolated Rust tests cover all three identity fields, ordering, per-receiver dedup, absent/unknown registrations and atomic invalid-receiver rejection. Native differential boundary fixtures remain unavailable. Full-host admission and complete gameplay parity remain false. The independent raw **89,198-byte Save→Load failure** stays unclassified and unmasked; no AI/UI/cache namespace exclusion is introduced.
