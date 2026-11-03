# Linux Player AI.Diplomatic saved wire

The bounded [Rust codec](../crates/c6-core/src/player_ai_diplomatic.rs) covers the complete current **saved wire** of `AI::Diplomatic` version 18, including its transitive saved records. This is distinct from gameplay `Player::Diplomacy`. It does not reconstruct native relationships, resolve definitions, run policies or execute diplomacy actions. Full Player/world import, externally supplied decisions, exact actions and turns remain unsupported. Diplomacy and gameplay RNG remain in scope; no planning-only exclusion is established. The recorded **89,198-byte raw Save→Load difference remains unmasked**.

The [read-only audit](../tools/linux/player_ai_diplomatic_audit.py) gates Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, complete function extents, resolved branch bindings, archive save/load virtual slots and matching DWARF. Its [report](../evidence/linux/player-ai-diplomatic.json) compares the two canonical original Core streams with their SHA-gated existing owner traces. It executes no original code and writes no private artifacts. Discovery attribution is [cru121's reference](reference.md), pinned revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; target ELF symbols, DWARF and bytes establish these contracts.

## Complete current grammar

All words are little-endian. `i32` preserves signed identities and raw enum ordinals; definition hashes and unsigned values preserve all 32 bits. Q8 fields retain their signed raw integer representation without float conversion. Every vector and hash array has a `u32` count. Nested inline native allocation capacities produce no extra wire fields. Strings are `u32` byte length followed by exactly that many char bytes, without a terminator or encoding validation. The codec represents these as byte vectors, including embedded zero and non-UTF-8 bytes.

Main writer `0x205b820` (615 bytes) and reader `0x205ba90` (2,254 bytes) establish this order after version 18:

1. Player ID; minimum force to notice and minimum troubling force Q8.
2. Diplomatic and base diplomatic Q8 score vectors; vector of visible Q8 score vectors.
3. StateVector; ActionEvalVector.
4. Number of majors met; player threats Q8 vector; major-threat bool vector; player trusts Q8 vector.
5. Managed Control handles; best potential friend Player ID.
6. Per-partner vectors of DiplomaticAction hash arrays for preference `i32`, worth Q8 and closed bool state, in that order. Each inner hash array retains its count and ordered `(u32 hash, value)` entries.
7. Requested TradeItems; excluded TradeItems; favor traded this turn `i32`; done-off-turn bool byte.

The following table lists every nested saved record. Only the current version is accepted. The audit's machine-readable `wire_schemas` records every individual field in order.

| Record | Version | Wire after version |
|---|---:|---|
| ControlHandle | 1 | Player ID `i32`, Control ID `u32` |
| StateVector | 2 | vector of DiplomaticState, isUpdated bool |
| DiplomaticState | 5 | owner/opponent IDs, state-name bytes, isUpdating/isActive/hasChangedState bools, diplomatic bonus Q8, state-definition hash, allowed/response/desired action-hash vectors, transitions, triggered transitions |
| DiplomaticTransition | 2 | diplomacy-player/opponent IDs, current-score Q8, transition ordinal, next-state bytes and ordinal, required/allowed min/max bool-value pairs, mustTransition bool, transition-score Q8, throttle turns, condition/transition hashes, transition-action/gained/lost action-hash vectors |
| TriggeredTransition | 1 | next-state-name bytes, next-state ordinal, trigger ordinal |
| ActionEvalVector | 2 | vector of DiplomaticActionHandler, isUpdated bool |
| DiplomaticActionHandler | 5 | owner/opponent IDs; conditional desirability/cost hash arrays; alliance/war/war-penalties Q8; isInitialized bool |
| TradeItem | 1 | GameTurn and OtherPlayer `i32`; ItemType, ItemSubType and ItemValue `u32` |

The two Handler hash arrays exist exactly when **both** owner and opponent differ from `-1`; all three Q8 values and isInitialized follow in either branch. `Option<Vec<HashEntry<i32>>>` preserves this structural distinction; encoding refuses a value whose optional arrays disagree with the two IDs. Current ActionEvalVector and Handler version headers use the native signed-int overload, while other current headers use unsigned words. Positive current constants have identical bits; unknown headers, including negative raw patterns, are rejected.

Native bool object I/O is byte-preserving here. `OStream::WriteObject(bool const&)` (`0x1fe7900`, 14 bytes) passes the original address and byte count one to the stream. `ReadObject(bool&)` (`0x1fe81a0`, 75 bytes) reads one byte into the object. Wrapped bool save, sequence callbacks and indexed bool-array writers pass the object's address through these overloads without truth-value conversion. `BoolByte(u8)` therefore preserves noncanonical bytes and makes no native truth-value claim.

## Native Read effects remain separate

Native main Read resolves the Player through Globals, calls `Diplomatic::InitializeLists` (`0x205c360`, 4,133 bytes), reads the payload, visits saved states through `DiplomaticState::InitializeSignals` (`0x20961c0`, a one-byte return body in this target), and invokes `Diplomatic::InitArchive` (`0x205d390`, 414 bytes). The codec performs none of those live-object operations.

Nested Transition Read resolves its saved diplomacy player to a live AI object when not `-1` and calls global `DiplomaticSupport::SetupFunctions` to restore function pointers from hashes. Valid-pair Handler Read calls Initialize, establishes evaluator dependencies, initializes desirability to zero and cost to Q8(1000), and then remaps known definition hashes. Unknown hashes are consumed and discarded by native mapping; duplicate mapped entries overwrite earlier values. The wire codec instead retains original hashes, duplicates, order and values. It does not claim native remapping equivalence. Native zero-length string Read skips Assign/Clear and can retain an existing live value; the wire codec records an empty byte string. Historical State versions can invoke ChangeState and other compatibility defaults; legacy formats are unsupported.

These concrete lifecycle dependencies preclude treating native Read as a byte copy. Neither direct call inventory nor the AI namespace proves RNG isolation or semantic irrelevance.

## Bounds and validation

Strict decoding rejects trailing bytes; prefix decoding returns the exact consumed bytes and aggregate item count. Limits use `unit::Limits`. Each vector element, outer row, hash entry and string byte consumes one dynamic item at its own container level. Nested elements share a single checked ledger. Every read checks byte limits and remaining minimum wire size before allocating; owned vectors use checked size arithmetic and fallible `try_reserve_exact`. Encoding measures the complete output and aggregate items first, fallibly reserves exactly that output, and then writes. These are the same allocation/accounting policies as the accepted CultureAi codec, implemented locally without inherited decoder allocations or hidden budgets.

Small typed source carriers exercise every nested record, raw bool bytes, signed/Q8 extremes, raw hashes, binary strings, all four Handler identity-presence branches, every truncation, all nested version gates, hostile counts and exact/one-short byte/item budgets. Actual records use bounded structural truncation cuts: every count/version field, container boundary, and scalar boundary/interior in the first/last repeated rows. This avoids quadratic testing of every byte in large repeated records.

The actual trace hooks **DiplomaticState, DiplomaticActionHandler and DiplomaticTransition**; counts are checked at their true descendant depth. It does not hook TradeItem or AI.Economic, so their absence is reported through the real inventory. The parser consumes the complete grammar without owner offsets. Diplomatic ends at the next Economic v4/Player header. The next actual sibling hook is Governors, after an unhooked Economic gap; that gap is comparison evidence, not a production parsing rule.

For a committed exported source checkpoint:

```sh
python3 tools/linux/player_ai_diplomatic_audit.py --source-commit COMMIT --output target/linux/player-ai-diplomatic.json
```

The driver refuses an archive without the committed module export, builds the unchanged Git archive, runs the focused Cargo tests and strict all-target Clippy, compares every original record's typed scalar projection and exact re-encoding, and checks every archived source against Git before and after the build. No source is injected into the archive.

The committed report validates exported source checkpoint `f09c9d2cd66d51426265f36bb45537297e851aad`: three focused Cargo tests and strict all-target Clippy pass. All **86 original Diplomatic records** exactly roundtrip across **7,735,964 bytes** and **1,269,842 aggregate dynamic items**; the independent typed projection digest is `6517532954482725705`. The actual corpus contains 5,504 DiplomaticStates, 5,504 Handlers, 3,844 Transitions, 5,508 TriggeredTransitions and 1,212 TradeItems, with **286,704** bounded structural truncation cuts. The audit verifies all 1,256 archived regular sources before and after the unchanged build. Original initialized native Read/action/turn differential acceptance remains a validation gap.
