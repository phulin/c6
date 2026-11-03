# Concrete Linux manager world inputs

`c6_data::unit_operation_manager` reconstructs disabled-operation IndexLists after the original Read reset, builds tactical-phase definitions from identity-gated Linux rules, and reads Player identity/human/turn-active facts from owned typed inputs. The source checkpoint is `7e6421d`. These are concrete inputs for the [manager predicate contract](linux-unit-operation-manager.md), not a complete Manager provider or native Manager acceptance. Complete gameplay state, actions, all RNG and turns remain unimplemented.

The pinned XP2 module SHA-256 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; the matching Linux host is independently gated. Discovery uses the target's instructions, symbols and embedded DWARF. Raw method evidence stays in `../c6-ghidra/evidence/linux-unit-operation-manager-world/source/`. Existing [registry](linux-operation-registry.md), [Unit](linux-unit-archive.md), [rules](rules-data.md) and [Player header](linux-player-header-control.md) contracts supply the established codecs and identities. Reference attribution remains in [reference.md](reference.md).

## Disabled-operation restoration

Original IndexList Read (`0x2c3fac0`) first invokes Reset. Reset reaches `0x2c3fbd0`, writes the default/purgatory byte to false and assigns false to exactly the actual operation-definition count. ReadIndexHashArray (`0x2c3fb20`) then consumes ordered `(hash,bool)` pairs. HashToIndex (`0x20e9f70`) calls the actual definition collection's Find and GetCollectionIndex. A miss still consumes its bool into a temporary and leaves the reset vector unchanged. A repeated resolved index overwrites its earlier value. This applies independently of saved pair count; the number of pairs does not determine live capacity.

`OperationDefinitions::from_native_order` requires the genuine ordered collection index/hash list and validates every index against its position. It reuses the verified native index-first lookup and last-inserted duplicate-hash precedence. A key below collection count means a direct index even when its bits also match a definition hash. Database rowids, operation hashes and live runtime IDs remain distinct. No SQLite order is used to invent operation IDs.

`restore_unit_disabled` reads the complete typed Unit's saved disabled pairs and allocates its reset vector once. Missing definitions stay false because the original reset explicitly initializes them, rather than because a query supplied a default answer. `DisabledOperations::is_disabled` performs the original FindIndex again on the incoming runtime ID bits, then reads the initialized value. Unknown keys return the original invalid-index false result. Even negative signed input bits retain lookup semantics; they are not blanket rejected if their unsigned representation identifies a real definition hash. Definitions and values share a borrowed lifetime, preventing lookup against a different table. Queries allocate nothing.

The model is specifically a fresh Read restoration. Native actions or setters can subsequently change the live IndexList; a restored snapshot cannot silently replace those changes. The current source replay contains **1,262 Units × 63 definitions**, consumes **79,506 saved pairs**, and reconstructs zero true slots. An independent Python source model agrees with every resulting vector; each complete Unit remains byte-for-byte unchanged. This is not a comparison with separately observed live IndexList bytes. Synthetic tests cover duplicates, unknown keys, duplicate definition hashes, small-key index priority, empty saved pairs and signed-bit boundaries; native fixtures for those branches remain unavailable.

## Tactical phases and rules

The original Phase InitializeTurnPhaseData (`0x2a54700`) filters definition objects by actual configuration TurnPhaseTypeHash and sorts by signed PhaseOrder. The runtime `current_phase` is an index into that selected vector, not a database rowid, phase hash or general definition index. `PhaseDefinitions::from_rules` uses the already identity-gated Linux rules tables, preserves native filtering, and sorts only when selected orders are unique. Equal selected orders return unsupported because the native sort's tie permutation has not been implemented. `from_native_slots` can instead accept an actually observed ordered vector, including proven null slots.

Native phase/segment builders select their SQL rows in intrinsic rowid order. ActiveSegment resolution uses the exact join:

```sql
SELECT T1.rowid FROM TurnSegments AS T1
INNER JOIN TurnPhases AS T2 ON T2.ActiveSegmentType = T1.TurnSegmentType
WHERE T2.rowid = ? ORDER BY T1.rowid ASC LIMIT 1
```

The original phase allocation initializes its active pointer to null, and the resolver assigns it only when the joined segment object exists. The Rust projection preserves that null branch. Canonical SQL integer boolean cells build the segment's original raw flags: strategic bit 0, tactical bit 1 and turn-unready bit 2. Noncanonical cells or unsupported scalar ranges reject rather than approximating database conversion. Windows rules profiles reject.

IsAllowTacticalCommands (`0x2a54f30`) ignores its PlayerTypes argument. Current phase `-1`, a proven null phase, or a proven null active segment returns true; otherwise the method returns segment flags bit 1. Other negative or out-of-range phase indexes are unsupported because the native vector access is unchecked. The adapter does not turn unavailable definitions into a native true branch. `from_processor` reads the actual typed saved `PhaseProcessor.current_phase`; no permission answer is an input.

The compiled replay loads the genuine activated diagnostic **506-table** rules snapshot through its full identity gate. It reconstructs all **14 phase slots across four possible configured phase types**, with ten tactical-false slots, and matches an independent SQLite projection. This enumerates rule possibilities. The actual current/configured runtime phase vector for a full Manager capture has not yet been observed under the same context, so the replay does not select a guessed active configuration.

## Player and control fields

`WorldPlayer::resolve` requires captured actual nonnull manager-slot facts and an owned Player of the same identity; it adds no Manager IsValid gate to original GetPlayer. The retained Player view returns its actual ID and obtains human status from captured configuration SlotStatus equal to 3. Missing slot facts, Player records or slot status remain errors. It reconstructs all 43 imported Player identities/human values from the accepted placement input, including its one human Player.

Original Player::IsTurnActive (`0x2021030`) reads `this+0x290`. DWARF identifies wrapper `m_bIsTurnActive` at `+0x280`, with its boolean value at `+0x10`. The complete Player47 writer/reader and wrapper methods prove that this is the **third ordered header boolean**, `ordered_flags[2]`. Writer call `0x2b4ef4c` and reader call `0x2b4f897` both serialize that wrapper. The source replay structurally locates and decodes the first canonical Player header, checks ownership, and supplies its saved turn-active value. This does not prove that later load callbacks or actions preserve that value in live memory, and does not invent headers for the other 42 Players.

`LiveControlFields` accepts the actual raw automation-processing Player enum and canonical raw Game pause byte. Their getters preserve the target's read semantics: processing enum at module `0x3c06598` through `0x2b5c3c0`, and Game `+0x228` through `0x281b720`. No default constructor or fallback is supplied. The current compiled replay does not contain a new native observation of both raw fields; unavailable controls must remain unsupported.

## Capture and integration limits

The new metadata capture needs actual ordered operation definitions and runtime identities; current live disabled vector count/default/bytes; configured phase hash plus actual ordered phase slots, rowids, signed order and nullable segment raw flags; actual current-phase enum; genuine pause/automation raw fields; and live Player ID/turn-active/configuration facts. These are raw state/definition facts, not expected predicate answers. A same-context capture must establish their shared configuration and owner lifetimes.

The native phase vector at module `0x3c05f08` is 32 bytes: begin/end/capacity pointers at 0/8/16 and allocator at 24. Its declared bool IndexList is also 32 bytes: vector storage at 0, unsigned size at 16, and default byte at 24. Original DWARF declares operation-definition size/GetCount as **uint32_t**, despite the implementation computing pointer differences in a 64-bit register; FindIndex and HashToIndex return int32_t. A native observer must use those declared widths and bound actual extents before traversal. Global GetScriptSystem (`0x2171660`) only loads the current pointer and has no lazy construction, calls or mutation.

Actual configuration GetTurnPhaseType (`0x28e4870`) calls the original IsNetworkMultiplayer and typed-variant provider; the nonnetwork path returns the original single-player hash. Its delegated provider ABI/binding still requires the native metadata helper's independent admission. No raw configuration choice is inferred from that constant alone.

This module does not replace the remaining explicit Manager providers: actual handler interruption, script-system/args allocation and destruction, ordered callback effects and engine locking, current Plot lookup and complete requested-handler effects. It adds neither a fabricated no-script environment nor a default legal gate. The compiled reconstruction combines separately identified original inputs for source-model checks; it is not a newly accepted full Manager native profile. The frozen handler candidates and existing native profiles remain unchanged.

Reproduce without original execution:

```sh
python3 tools/linux/unit_operation_manager_world_audit.py --output target/linux/unit-operation-manager-world.json
cargo test -p c6-data unit_operation_manager
C6_MANAGER_RULES=/path/to/genuine/activated/export cargo test -p c6-data genuine_rules_phase_provider_reconstruction -- --ignored
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash tools/check/c_abi_smoke.sh
```

The [audit](../evidence/linux/unit-operation-manager-world.json) gates full original methods, call edges, field/bit instructions and declared layouts, then builds an immutable source archive at `7e6421d` and verifies every extracted source file before compiling the separate replay. Private reconstruction rows and executable/source identities remain recorded. The raw **89,198-byte** Save→Load failure remains unclassified and unmasked. No whole-action benchmark or complete state/action/RNG/turn claim is made.
