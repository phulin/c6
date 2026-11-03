# Linux Military Operation saved-wire contract

This implements the concrete transitive Operation records inside the bounded
Military saved-wire codec. It does not implement native loading, policy choices,
gameplay state transitions, actions or turns. Unsaved native fields remain outside
this wire representation; their absence from these serializers is not evidence
that they are irrelevant to gameplay or may be excluded from an exact simulator.

The original Linux XP2 module is pinned to SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
No original code is executed by this audit. Original artifacts and raw
disassembly are not copied into this repository.

| Record | Current version | Write address | Read address |
| --- | ---: | --- | --- |
| TeamRequirement | 2 | `0x215bc20` | `0x215bca0` |
| TeamDefinition | 2 | `0x215bd40` | `0x215bdc0` |
| OperationDef | 7 | `0x215be50` | `0x215bf90` |
| OperationTarget | 1 | `0x215c180` | `0x215c1f0` |
| OperationTeam | 3 | `0x215c250` | `0x215c340` |
| Operation | 6 | `0x215c490` | `0x215c5d0` |

Each record starts with its little-endian u32 version. Only the current versions
are supported: older versions fail explicitly rather than supplying defaults.
The authoritative ordered field/type grammar is `SCHEMAS` in
`tools/linux/military_operations_contract_audit.py` and is also emitted in
`evidence/linux/military-operations-contract.json`. The Rust records preserve this
same field order directly, with no polymorphic dispatch or opaque sections.

`i`, `u`, `s` and `b` mean little-endian i32, u32, i16 and a raw one-byte bool
storage value. Coordinates are two i32 values. Component IDs are i16 owner, i16
component type, u32 ID. Vectors store a u32 element count followed by each element,
including separate counts for nested vectors. Float fields retain their u32 bit
patterns; this record family contains no fixed-point Q8 fields.

`SavedString` stores a u32 byte count and exactly that many raw bytes, without a
saved terminator, UTF-8 conversion or normalization. This follows the narrow
string writer at `0x1ff5430` and reader at `0x1ff4d20`. `OperationPositions`
contains a saved u32 count equal to six followed by six coordinates. The current
team writer passes six to `WriteArray<PlotCoord>` at `0x2165a30`; other counts are
rejected. String bytes and all counted sequence elements charge the shared item
budget as well as the shared byte budget.

The team's needed unit is the raw saved u32 hash produced by
`WriteIndexToHash<Unit>`; no rules index is guessed or looked up while transporting
the wire. `OperationDef::NameHash` and requirement tag hashes are direct saved
i32 fields. The native FAutoVariable wrappers delegate to their contained values,
so no wrapper headers or archive metadata are inserted into these records.

Important DWARF members absent from the current saved sequence include the team's
player/definition pointers, per-requirement counts, targets and formation pairs,
and the operation's known-enemy knowledge lists. The reader performs native
pointer/relationship restoration beyond reading the saved fields. The wire codec
does not claim to reconstruct those relationships or to execute the operation.

Reproduce source evidence with:

```sh
python3 tools/linux/military_operations_contract_audit.py
```

The audit checks the whole original ELF identity before inspecting 32 symbol
extents, their byte hashes, 257 direct call/jump bindings and the DWARF output for
all six records. Its default mode compares the result to checked-in evidence;
`--output PATH` writes the metadata report. Shared helper implementation contracts
and the native saved-corpus comparison are additionally checked by the enclosing
Military audit. Native trace offsets serve only to validate comparison cuts;
they never define production parser boundaries.

The synthetic nested codec fixture checks raw non-UTF-8 strings, float NaN/sign
bits, noncanonical bool storage, nested counts, exact and one-short budgets, every
truncation, and wrong fixed-position counts. These are codec boundary checks;
they do not replace the enclosing Military native-corpus comparison or establish
gameplay equivalence. Complete gameplay state, action execution and turns remain
unimplemented. Unknown save bytes remain unclassified, including the previously
reported 89,198-byte raw Save→Load difference.
