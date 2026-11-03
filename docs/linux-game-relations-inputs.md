# Owned relationship inputs for loaded gameplay queries

`c6_data::game_relations` constructs actual War/Alliance definitions and admits
the existing source-attributed Game.DiplomaticRelations prefix into LoadedState.
Unit.GetMaxMoves/HasMoved now reads this owned state, which remains current across
unrelated Unit queue/activity/timer mutations. No pre-action alliance snapshot,
predicted future revision or precomputed alliance predicate is used.

This closes the concrete missing inputs in the earlier movement query fixture.
It does not implement relation setters/upkeep, complete diplomacy, full SkipTurn
reporting, or complete state/action/turn parity. The current owner prefix is
bounded; the original image and every unclassified suffix byte remain retained.

## Definition and load contracts

The [definition audit](../evidence/linux/game-relations-definitions.json) pins the
same exact Linux XP2/host identities as the existing rules and Core audits. It
checks seven enum/lookup bodies and 34 native collection-builder bodies. Raw
source stays in `../c6-ghidra/evidence/linux-game-relations-definitions/`.

| Collection | Native SQL getter / collection build | Exact source selection |
| --- | --- | --- |
| Alliances | `0x21e8f90` / `0x224ad00` | All Alliances rows, ORDER BY rowid; AllianceType is selected column1. |
| Wars | `0x221f7c0` / `0x22609c0` | All Wars rows, ORDER BY rowid; WarType is selected column3. |

There are no additional source filters in these two SQL queries. Their native
builders set each collection index to the collection's size before append. The
adapter reuses the established native-order registry helper and actual admitted
506-table rules snapshot, checking the existing immutable resource/configuration
gate. SQL row IDs, collection indexes and type hashes remain distinct.

WarTypes Read calls ValidateHash (`0x2d514d0`): resolve the supplied value through
WarDefinitions.Find and return the found definition's **type hash**. AllianceTypes
Read calls HashToIndex (`0x2bf6fc0`): resolve through AllianceDefinitions.Find and
return the definition's **collection index**. A miss becomes -1. Both concrete
Find methods first accept an unsigned value below count as a direct ordinal;
otherwise they search by unsigned hash. Native insertion puts later duplicate
hashes first, so hash lookup resolves the most recently inserted duplicate.
The [core initializer](linux-game-diplomatic-relations.md) implements these
boundaries; it does not merely test hash membership or treat saved alliance hashes
as live enum indexes. The admitted dataset has 15 War definitions and five
Alliance definitions.

`Definitions::from_rules(state, rules, bound, rules_report)` checks the Bound's
actual destination binding as well as the registered rules/resource identities.
It owns complete native-order definition inputs. Its
`install_attributed_snapshot(state, original_owner_trace, limits)` supports the
existing ff31 source fixture explicitly:

1. Verify the entire 80,657,462-byte source image and its binding against ff31.
2. Verify the original decompressed owner-trace hash, then find the unique
   Game.DiplomaticRelations Save1 frame under the actual Game root.
3. Verify the complete 195,621-byte owner and its 121,005-byte decoded prefix
   hashes. The actual trace supplies `[72493107,72688728)`; no ID-pattern scan or
   fixed-offset general Core parser is introduced.
4. Ask LoadedState to decode that witnessed image slice once, apply the admitted
   definition initialization and own the resulting mutable relation component.
   Replay or installation after advancement rejects.

The witness records SavedOutput provenance independently of the decoder's
Read/Write framing role. It does not claim that a native Read callback was
executed by Rust. This fixture adapter does not generalize its owner attribution
to other Core images without their own accepted framing evidence.

## Complete query inputs in the actual fixture

The [new offline check](../evidence/linux/game-relations-query-input-check.json)
uses an isolated immutable copy of every required public source and literal
compile-time include, recording all hashes and WIP dependencies. No original game
code is executed. It admits observed controls through the existing one-shot
adapter, which requires the exact associated full ff31 image and original
control/resource/configuration preimages. Manager validity is not inferred from
saved roster membership.

The checker initializes the owned relation prefix from the actual saved owner,
compares **all 2,016 War values** with independent closed native metadata, then
drops the captured war sidecar. That sidecar serves as a validation baseline;
it is not another mutable gameplay authority after owned installation. Original
changed-turn words remain intact. Twenty-six saved non-absence alliances resolve
through the actual five Alliance definitions.

With those inputs and the actual Unit definition/modifier state, the result is:

- **1,262/1,262 Unit maximum-movement/HasMoved queries**, with no unavailable-input
  results; 844 evaluate HasMoved=true.
- **2,524 strict-Q8 boundary checks** around the computed maxima. Equal/excess
  remaining movement returns false; one raw-Q8 step below returns true.
- Original actor owner0/id15794187 computes maximum3 from real fields/rules,
  remaining256 raw Q8 and HasMoved=true.
- An unrelated Unit timer mutation and revision increment preserve owned
  relationship authority and the query result. Replay installation still rejects.
- Unit wire state remains unchanged. The 74,616-byte owner suffix remains
  unclassified in the original retained source image.

The [earlier 1,059-success report](../evidence/linux/unit-movement-state-check.json)
and its 203 precise missing-input cases remain frozen. This new report closes
those inputs using actual same-snapshot controls and owned initialized relations;
it does not fill defaults or reinterpret the earlier failures as passes.

## Native comparison limit

The current vector decoder has independent saved-record comparisons and the new
War values have a closed native metadata comparison. **Native runtime Alliance
indexes/IsAllied results and direct Unit.GetMaxMoves/HasMoved outputs have not been
captured for differential comparison.** The 26 alliance results and all 1,262
movement results are source-backed Rust computations, not newly observed native
query returns. The owner Read suffix uses disjoint wrapper receivers, while
full enclosing Game post-load/transitive callback equivalence remains unclosed.
Neither source checks nor these fixture queries establish full action parity.

Reproduce:

```sh
python3 tools/linux/game_relations_definitions_audit.py --private-output ../c6-ghidra/evidence/linux-game-relations-definitions --output target/linux/game-relations-definitions.json
python3 tools/linux/game_relations_check.py --output target/linux/game-relations-query-input-check.json
cargo test -p c6-data unit_movement_state --lib
cargo clippy -p c6-data --all-targets -- -D warnings
```

No native execution, observer/profile expansion, native save production or binary
save rewriting is required by this input closure. Gameplay reports/subscribers,
all actual diplomacy changes, action legality, complete RNG and external player
decisions remain required. The separate raw 89,198-byte Save→Load difference stays
retained and unmasked.
