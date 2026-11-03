# Linux Player.GreatPeoplePoints wire boundary

`player_great_people_points::GreatPeoplePoints` implements the complete current
version 13 saved wire record. The pinned Linux writer is `0x2b10cb0` (462 bytes),
reader `0x2b10e80` (1,035 bytes), and version word `0x2f617c4`. The Player writer
calls it at `0x2b4f132` through PLT `0x1e524a0`. The next component is Influence:
call `0x2b4f145`, PLT `0x1e87530`.

The complete grammar, in native order, is:

1. Version u32 and owning PlayerTypes raw signed i32 ordinal.
2. Three Great Person class hash/raw signed Q8 tables: points total, points
   earned this turn, bonus points per turn.
3. Free Great Person points i32, points refund percent i32, no-patronage-faith
   bool byte, no-patronage-gold bool byte.
4. Yield hash/signed integer patronage-discount table, Great Person class
   hash/signed integer bonus-points-percent table, yield hash/raw signed Q8
   modifier-per-earned-Great-Person table.
5. A u32-counted vector of yield changes. Each entry has version 13, yield
   definition hash u32, Great Person class definition hash u32, resource
   definition hash u32, and signed integer amount i32 (20 wire bytes).
6. A u32-counted vector of earned boosts. Each entry has version 13, Great Person
   class definition hash u32, tech-boost bool byte, other-players bool byte
   (10 wire bytes).
7. A u32-counted vector of Great Person class definition hashes u32.

Each table is u32 count followed by ordered `(u32 definition hash, i32 value)`
pairs. All duplicates, hash bit patterns, signed widths and raw Q8 values are
preserved. Definition enums in the nested records and final vector dispatch
through `WriteIndexToHash`; they are not SQL indices or raw enum ordinals.
There are no strings, fixed arrays or other transitive records in this grammar.
The nested writer versions share the same current-version word. The current
reader follows the matching grammar; legacy branches are unsupported.

DWARF supplies the named members and nested carrier types. The audit pins
concrete PLT calls, FAutoVariable virtual save/load slots, serializer method
bytes, nested type layouts, and module/host identities. Native Initialize
(`0x2b11290`) assigns the parent/archive ID and calls Reset (`0x2b11850`). The
native reader remaps definition hashes and edits native variables. This codec
performs saved wire transport only; native reset, callbacks, remapping, gameplay
and RNG effects are not reproduced. This Player record establishes no grammar
or state coverage for the global Great People manager. Recruitment and all
other player choices remain external policies.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode` and
`encode_with_items` use `unit::Limits`. Each table pair or sequence entry charges
one aggregate item; fixed scalars and nested versions have no item charge. Every
declared count checks arithmetic, aggregate items and minimum remaining wire
before fallible typed reservation. Dynamic typed storage uses at most 16 bytes
per charged item plus fixed struct storage, independently of the wire-byte cap.
`max_bytes` measures wire, not cumulative heap allocation. Encoding checks the
whole record before allocating output. All version words and bool bytes are
strictly validated.

```sh
python3 tools/linux/player_great_people_points_audit.py --source-commit COMMIT \
  --output evidence/linux/player-great-people-points.json
```

The driver and module must be committed together before this audit builds from
one isolated source archive. Owner trace cuts are validation inputs only; the
production codec has no offsets or owner traces. Every full component record in
both canonical streams must roundtrip, pass exact byte/item caps, reject
one-short caps and reject every truncation. Source tests exercise nonempty
nested carriers, duplicate/extreme hashes, signed extrema, invalid outer/nested
versions, invalid outer/nested booleans, hostile counts and trailing input.
No original game execution, Ghidra project writes or proprietary artifact writes
are used by this audit.

Complete Player loading, Core/world import, exact gameplay state, action
execution and turns remain unimplemented. Unknown save bytes remain unclassified
and unmasked, including the recorded 89,198-byte raw Save→Load failure. Offline
saved-wire comparisons do not establish native-Read or gameplay parity.
