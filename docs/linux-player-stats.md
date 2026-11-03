# Linux Player.Stats wire transport

`player_stats::Stats` represents the pinned Linux XP2 current writer v27
(`0x2f678cc`). The writer is `0x2b7c710` (0x3f0 bytes), and the reader is
`0x2b7cb00` (0xaaf bytes). Linux symbols, DWARF and hash-gated original bytes
establish the schema. The Player writer calls Resources, Stats, Techs, Treasury
and Units in that order (`0x2b4f0ad` through `0x2b4f0f9`).

The record contains 22 signed i32 statistics; a recursive TrackedValue for
diplomatic victory points; last/current/lifetime favor and two raw Q8 science
values; ordered Unit and Project hash/i32 arrays; an ordered i32 wonders-lost
plot vector; a Yield hash/Q8 array; seven count-prefixed fixed arrays of 64
i32 counters; and sixteen signed i32 disaster counters. The Rust declaration
follows the actual writer order, which differs from native member order.
The seven fixed arrays have a literal writer count of 64; other counts do not
silently pad, truncate or change their meanings. Definition hashes and duplicate
pairs remain ordered raw u32 identities. Nothing resolves hashes to SQL ordinals.

TrackedValue v1 has its own typed codec. Its steps carry priority i32, ID u32,
Localization::String, and another TrackedValue in that wire order. Both optional
Q8 bounds and all nested values remain exact wire carriers. The nested codec
limits tracked and localization recursion to depth 64 as an explicit resource
policy, not a native gameplay restriction. Native Read resets
and overlays initialized fields; older-version migrations, definition remapping,
archive edits and live gameplay effects are separate from this transport.

`decode_prefix_with_items` and `encode_with_items` report the aggregate item
charge for composition. Fixed arrays charge all 64 elements; variable vectors
charge their entries; nested codecs contribute their exact charges. Wire-byte
limits apply cumulatively. Every vector checks its addressable allocation layout
and uses fallible allocation; cumulative typed heap size is not substituted for
the wire-byte limit. Strict decoding rejects trailing bytes and unsupported
versions. The parser does not use owner traces or fixed record offsets.

Run the read-only audit after committing the implementation and dependencies:

```sh
python3 tools/linux/player_stats_audit.py --source-ref HEAD --output evidence/linux/player-stats.json
```

The audit builds a fresh immutable Git archive of the selected commit, records
that exact source pin and executable hashes, and compares every Stats owner
record in both existing canonical streams. Hash-gated owner traces supply only
offline validation cuts. The audit also runs source boundary tests for nonempty
vectors, duplicate hashes, signed extremes, every truncated prefix, trailing
bytes, bad versions and booleans, fixed-array framing and exact/one-short caps.
The recorded isolated run passed eight combined Stats/TrackedValue boundary
tests and round-tripped all 86 saved Stats records (282,098 total bytes) under
exact and one-short budgets. This comparison is not a new native mutation oracle. Original game code is never
executed, and no private artifact is modified.

Discovery attribution: pinned `cru121` revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`; only Linux target evidence supplies
acceptance. No proprietary code or assets are included. Complete Player loading,
world import, gameplay state, action execution and turns remain unimplemented.
The recorded raw Save→Load difference remains 89,198 bytes, unmasked and
unclassified. The next Player component is Techs.
