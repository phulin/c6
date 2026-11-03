# Linux Player.Trade wire boundary

`player_trade::Trade` implements the complete current version 35 saved wire
record. The Linux writer is `0x2b92910` (1512 bytes), reader `0x2b92f00`
(3826 bytes), and current-version word `0x2f68bbc`. The Player writer calls
Trade at `0x2b4f11f` through PLT `0x1e397e0`, resolving to that concrete writer.
The following component is GreatPeoplePoints: call `0x2b4f132`, PLT `0x1e524a0`.

The grammar, in native order, is:

1. Version i32, owning PlayerTypes ordinal i32, fifteen named signed i32 fields.
2. Two u32-counted vectors of one-byte booleans (civic, then tech boost receipts).
3. Thirteen yield hash/Q8 tables, techs/civics-per-yield i32, eight yield tables.
4. Exactly 64 per-player yield tables, without an outer count or player hashes.
5. Three improvement hash/nested-yield tables, the international specialty
   district yield table, four terrain hash/nested-yield tables.
6. Three signed i32 trade-disabled counts (major, minor, domestic).

A yield table is u32 count followed by ordered `(u32 definition hash, i32 raw
Q8)` pairs. A nested table is u32 count followed by ordered outer definition
hashes and complete yield tables. `IndexList` delegates to the same yield table
writer. Preserve every hash, negative raw value, duplicate and order; these are
not SQL ordinals, maps or floating-point values. The fixed per-player cardinality
comes from Reset's resize to 64 at `0x2b95854`; the writer iterates that vector
at `0x2b92d80`. No relationship authority is inferred from the saved PlayerTypes.
There are no localized strings or nested version words in this current grammar.

The reader resolves the saved player through Globals then calls Initialize
(`0x2b93e00`), which sets the parent, calls Reset and assigns archive ownership.
It remaps definition hashes and edits native variables. Wire decoding performs
none of those native operations. Assignment callbacks and any transitive RNG
or gameplay effects are not claimed as reproduced. This record is separate
from global trade managers and the Unit.MakeTradeRoute operation payload.

Strict/prefix decoding and `encode_with_items` use aggregate wire-byte and
collection-item limits. Items charge each boolean, each yield or outer nested
entry, and the 64 fixed player slots; fixed scalars have no item charge. Before
allocating, each count checks arithmetic, aggregate items and the minimum wire
required for its entries. Typed storage is separately bounded by charged item
count (at most the largest nested entry per item, plus the fixed struct);
`max_bytes` is a wire cap, not a cumulative heap cap. Reservations are fallible.
Encoding measures the entire record and checks both caps before output allocation.

The audit gates the pinned Linux module/host identities, method bytes, concrete
PLT bindings, DWARF and version. It builds the module and driver from one committed
archive, using no shared working-tree recovery. Owner traces supply validation
cuts only. Every full Trade record in both saved streams must roundtrip exactly,
pass at its exact byte/item caps, fail at one-short caps, and reject every prefix
truncation. Source tests cover nonempty carriers, duplicates, signed Q8 extremes,
invalid booleans, versions, hostile counts, trailing input and prefix decoding.

```sh
python3 tools/linux/player_trade_audit.py --source-commit COMMIT \
  --output evidence/linux/player-trade.json
```

This establishes saved wire transport only. Complete Player loading, Core/world
import, exact gameplay state, actions and turns remain unimplemented. Player
choices remain external policies. Unknown save bytes remain unclassified and
unmasked, including the recorded 89,198-byte raw Save→Load difference. No original
game code is executed and no private or proprietary artifacts are written.

Validated source commit `419c616` passes both isolated source tests and all 86
saved Trade records: 670,112 bytes per canonical stream, 1,340,224 bytes total.
Every native record is 15,584 bytes and 2,066 charged items. Exact caps pass,
every truncation fails, and one-short byte/item caps fail. The evidence records
the full source commit, archive, source/driver manifest and candidate identities.
These offline saved records do not constitute live native-Read or gameplay parity.
