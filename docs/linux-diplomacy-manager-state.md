# Owned diplomacy pending state

`diplomacy_manager::Manager` and `LoadedState::process_pending_diplomacy` implement the current Linux outgoing statement drain over owned state. This is **Game.Diplomacy.Manager**, distinct from Game.DiplomaticRelations. It closes a concrete state/provider interface, not complete diplomacy or an accepted whole SkipTurn.

The [source contract](linux-diplomacy-manager-source.md) pins the process-global lists and vector. `Initialize` does not clear them. Static construction and `Uninitialize` establish empty state at their own boundaries; ordinary save loading alone does not prove an empty manager. The historical execute-3 observation establishes an empty open list and pending-action vector only in that runtime. Its older report field `open_session_map_nonempty` actually reads the same list count used by the drain. No empty constructor or saved-image fallback is supplied here.

## Admission and stable identity

`PendingStateSnapshot` supplies the actual empty-open-session witness, ordered ever-alive Player handles, complete typed pending actions and an `ObservedSnapshotWitness`. `install_diplomacy_manager` requires the exact already-admitted dynamic witness, pristine state and no previously installed manager. Equal Core bytes from a different capture do not satisfy this gate. This is a consistency/provenance API; its capture adapter must validate the original preimages and boundary attribution. It is not cryptographic authentication of caller statements.

An empty witness can represent the exact native count getter, or a stronger passive list header with count zero and both links equal to the actual sentinel. Nonempty observation rejects rather than becoming an empty list. The 12-byte PendingAction record retains unsigned ID and signed from/to owners. Closed-session history and active-session routing are not represented by this bounded pending owner and are not declared empty.

Ever-alive iteration preserves the supplied allocation-slot order. `InitializeLists` scans the allocation-slot table, which is different from the player-ID pointer table. Sorting IDs or assuming serialized insertion order is incorrect. Installation checks complete handle membership against admitted actual pointer-presence facts and restored WasEverAlive flags, without changing order. Handles name stable owned Player objects by owner; native pointers remain capture evidence. The current library has no Player replacement API. Any future replacement must invalidate or rebind these handles explicitly.

## Execution and remaining dependencies

The drain copies handles once. It reads each current ID and configuration-backed IsAI once, then repeatedly starts at the current open-session head and removes the first outgoing statement. `IsAI` is `!IsHuman`, where the actual configuration predicate is SlotStatus==3. A missing current configuration is explicit unsupported input; saved roster presence is not a substitute.

Empty sessions execute without changing revision or invoking a statement provider. Nonempty state carries typed statement handles whose actual payloads belong to the required `ProcessStatement<LoadedState>` implementation. The statement is removed before the synchronous provider call; callbacks can populate an earlier session, and the next iteration sees it. Provider/decision errors preserve effects already performed, matching the existing diagnostic aftermath kernel. No default processor, native AI policy or successful synthetic decision is supplied. The diagnostic budget likewise follows the existing kernel and is not a native gameplay rule.

Source-only tests cover empty/nonempty witness distinction, handle order, pending-action ownership, removal order, same-context admission/replay rejection and reentrant callbacks ending at an explicit external-decision requirement. No genuine current Manager snapshot has yet been admitted by this checkpoint. To use the actual action fixture, the passive observer must supply its current open-list witness, full pending vector and ordered ever-alive identity/configuration facts, with its own closed runtime provenance. Historical execute-3 state must not be transplanted merely because ff31 bytes match.

Validation at this checkpoint: five new focused tests pass; the complete Core suite passes 413 tests with one existing fixture test ignored. Strict Core Clippy for all targets passes. These are source-only checks, not a genuine snapshot admission or native action comparison.

Run the [static audit](../tools/linux/diplomacy_manager_state_audit.py), focused Core tests and strict Core Clippy. The [evidence](../evidence/linux/diplomacy-manager-state.json) records exact source and target hashes. Complete Statement rules, diplomacy updates, gameplay callbacks/RNG and the next genuine external decision remain required for full action/turn support. The raw 89,198-byte save/load discrepancy remains unmasked.
