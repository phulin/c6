# Linux Player.Improvements saved wire

`player_improvements::Improvements` covers the complete current version 1 wire
record of the Player component: version u32 bits 1, unsigned u32 plot count,
that many ordered signed i32 plot indices, then the independent u32
`improvement_array_size`. Size is `12 + 4 * plot_count`. Each plot charges one
item. These indices are not definition hashes or SQL ordinals. The final size
word is preserved even when unequal to the vector length. No bool, Q8 value,
string, fixed array, nested version or localization field occurs here. Map Plot
improvement fields and City improvements are separate components.

The Linux writer is `0x2b1cd70`, reader `0x2b1cdc0`, local version word
`0x2f61dc0`. Player calls the writer at `0x2b4f17e` through PLT `0x1f74d10`.
DWARF identifies a 224-byte object: sync archive, vector auto-variable at 152,
unsigned array-size auto-variable at 192, parent pointer at 216. Saved vector
and size fields dispatch through verified vtable save/load entries. Vector
writer `0x1feff10` emits unsigned count then ordered signed scalars through
`0x1feffc0`, `0x1feff70`, `0x1fefff0`; reader `0x1fefe10` delegates to
`0x1fefe20`, which clears the existing vector and reads an unsigned count before
loading signed values. `GetImprovementPlots` at `0x2b1d110` copies those vector
values. The audit verifies full method bytes, concrete PLT bindings, virtual
bindings, scalar serializers and DWARF against the SHA-pinned Linux target.
The historical reference revision is discovery-only.

Native Read consumes the version without branching, clears and reloads the
vector and assigns the independent unsigned size through auto-variable load.
It does not call component Reset or Initialize, remap definition hashes, invoke
RNG or make player decisions. The wire codec does not reproduce native live
object mutation or synchronization effects. All player decisions remain
external; exact gameplay state and RNG remain in scope.

APIs are `decode`, `decode_prefix`, `decode_prefix_with_items`, `encode` and
`encode_with_items`, under `unit::Limits`. Decode checks the unsigned count,
aggregate items, checked full wire extent including the final size word, input
availability and typed storage bounds before fallible reservation. Encode
preflights representable count and complete bytes/items before reservation.
High-bit counts are unsigned and bounded. Unsupported versions and exceeded
caps fail explicitly; strict decode rejects trailing bytes. Production parsing
has no owner cuts, trace inputs, fixed fixture offsets or opaque skips.

Reproduce after committing both module and actual driver:

```sh
python3 tools/linux/player_improvements_audit.py --source-commit COMMIT \
  --output evidence/linux/player-improvements.json
```

The read-only audit gates GameCore and host hashes, builds one committed Git
archive, verifies driver/source/candidate identity, runs actual Cargo tests and
strict Cargo Clippy on its isolated codec harness, and roundtrips all 43 Player
records from each of both canonical streams. No Improvements owner hook exists:
Quests-end and enclosing Player-end provide validation context, preceding WMD
count grammar derives the record start, and Improvements count grammar
independently checks typed prefix extent. These are validation boundaries only.
Every truncation and applicable one-short byte/item cap is rejected. Source
fixtures cover nonempty ordered duplicates, signed extrema, independent size,
empty vectors, unsigned count caps, versions, trailing bytes and prefix APIs.
Bool tests are inapplicable. No original game code executes; private artifacts
are read only.

Next is Governors, Player call `0x2b4f191`, PLT `0x1f9ed50`. Full Player,
Core/world import, exact gameplay state, actions and turns remain unimplemented.
Unknown bytes remain unclassified and unmasked, including the recorded
89,198-byte raw Save→Load difference. Wire roundtrip does not establish native
Read effects or gameplay parity.
