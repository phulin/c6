# Linux Player.Influence wire boundary

`player_influence::Influence` implements the complete current version 22 saved
wire record. The pinned Linux writer is `0x2b1ed50` (660 bytes), reader
`0x2b1eff0` (1,356 bytes), and current-version word `0x2f61f44`. Player calls the
writer at `0x2b4f145` through PLT `0x1e87530`. The next component is Quests,
called at `0x2b4f158` through PLT `0x1e0a540`.

The exact grammar, in native serialization order, is:

1. Version u32, owning PlayerTypes raw signed i32 ordinal, points earned and
   base points per turn as signed raw Q8 i32, not-earning-points bool byte.
2. Six i32 scalars: points threshold, tokens per threshold, tokens to give,
   duplicate first tokens, tokens on first to meet, tokens on trade route started.
3. Tokens-received i32 vector, virtual-tokens-received i32 vector,
   disallow-tokens-from bool vector, yield-per-given-token hash/Q8 table.
4. Suzerain raw PlayerTypes i32 ordinal, suzerain-changed count,
   suzerain-unique-bonus-disabled count and levy-turn counter i32.
5. Yield-modifier-per-tributary and yield-per-tributary hash/Q8 tables.
6. Open-borders-count, levy-discount-percent-by-player, copy-luxuries-count and
   copy-strategics-count i32 vectors.
7. Duplicate-tokens-when-rival-government, duplicate-tokens-when-trade-route-to,
   duplicate-tokens-when-same-religion, suzerain-visibility-counter and
   levy-discount-percent i32 scalars.
8. Tokens-received-modifier and max-tokens-received i32 vectors.

Every vector is an unsigned u32 count followed by values. Every table is an
unsigned u32 count followed by ordered `(u32 yield definition hash, i32 raw Q8)`
pairs. Counts dispatch through the native unsigned-int serializers; signed
values retain every bit. All duplicates and ordering are preserved. Neither
player ordinal is a definition hash. No nested versioned records, fixed arrays
or strings occur. DWARF identifies all serialized carriers. The unsaved
initialization flag, parent pointer and giving-tokens-considered member are not
invented as wire fields. The three yield tables use definition hashes, never
SQL ordinals.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode` and
`encode_with_items` use `unit::Limits`. Fixed scalars charge zero items; each
vector entry/table pair charges one aggregate item. Every count checks arithmetic,
aggregate items and minimum remaining wire before fallible typed reservation.
Dynamic typed storage is bounded independently by `max_items`, at most eight
bytes per charged item plus fixed struct storage. `max_bytes` measures wire,
not cumulative heap allocation. Encoding checks the whole record before output
allocation. Versions and every bool byte are strictly validated.

The native reader calls Initialize (`0x2b1f540`) and Reset (`0x2b202b0`), uses
versioned legacy branches and remaps yield hashes. The codec supports current
saved wire only: reset, callbacks, definition remapping, RNG and gameplay effects
remain separate and unreproduced. Envoy choices and all other player decisions
remain external policies. This component establishes no separate grammar or
configuration authority for Game.DiplomaticRelations.

```sh
python3 tools/linux/player_influence_audit.py --source-commit COMMIT \
  --output evidence/linux/player-influence.json
```

The module and actual audit driver must be committed together before the driver
builds one isolated source archive. The read-only audit SHA-gates module/host,
serializer bytes, direct/PLT and virtual save/load bindings, DWARF and both
canonical saved streams. Owner cuts serve only as validation inputs. Production
code uses no offsets, traces or unframed skips. Source tests cover nonempty
vectors/tables, duplicate/extreme hashes, signed extrema, bool/version rejection,
every truncation, trailing input, hostile counts and exact/one-short caps.
The audit executes no original game code and writes no private artifacts.

Complete Player loading, Core/world import, exact gameplay state, action
execution and turns remain unimplemented. Unknown bytes remain unclassified
and unmasked, including the recorded 89,198-byte raw Save→Load failure. Offline
saved-wire comparison does not establish native Read or gameplay parity.
