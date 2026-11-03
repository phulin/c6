# Linux Player.Resources current wire

`player_resources::Resources` transports the complete current version 16 record
written by the pinned Linux XP2 `libGameCore_XP2.so` (SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`).
It is distinct from City.Resources and map resource fields. Matching host SHA-256
is `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.

The writer is `0x2b6b0f0`, reader `0x2b6b310`, and current version constant
`0x2f671d0`. Matching full DWARF identifies every object field; the audit pins
method symbol identities and original method bytes. Player writer call sites
`0x2b4f087` through `0x2b4f0f9` establish Espionage → Religion → Resources →
Stats → Techs → Treasury → Units. The next actual component is Stats.

All words are little endian. The record contains, in order:

| Wire field | Representation |
| --- | --- |
| version | u32, exactly 16 |
| free resource visibility | resource hash array of one-byte canonical bools |
| extra amenities per owned luxury, bonus resource | two i32 |
| full access one strategic | one-byte canonical bool |
| prevent harvest resource | i32 |
| resource ownership | resource hash array of counted i32 vectors |
| accumulated resource per turn | resource hash array of signed raw Q8 words |
| luxury allocations | counted unversioned CityResource records |
| reserved, banned, no-cap resources | three resource hash arrays of i32 |
| exported, imported, free resources extracted | three resource hash arrays of i32 |
| extra accumulation, stockpile cap modifiers | two resource hash arrays of i32 |
| unit, power resource demand per turn | two resource hash arrays of signed raw Q8 words |

A resource hash array is u32 count followed by ordered `(u32 hash, value)` pairs.
A vector is u32 count followed by its values. CityResource is u32 city ID then
u32 resource definition hash, without a nested version. The accumulated IndexList
wrapper also writes just its hash array, without a nested version. The native
ResourceTypes writer performs index-to-hash conversion; SQL ordinals are not wire
identities. Unknown hashes, duplicate entries, order, negative integers and all
Q8 bits are retained. Ownership values are counted, not assumed to have the
native reset default of three values.

`decode` rejects trailing bytes; `decode_prefix_with_items` reports consumed
bytes and aggregate vector/hash-array items for composition. Scalars and hash
words do not consume item budget; each array entry and each nested ownership
integer does. Byte and item limits aggregate across the complete record. Before
reserving a vector, decode checks item budget, minimum remaining wire bytes,
source availability, checked multiplication and the addressable allocation
layout. The wire byte budget is not incorrectly reused as a cumulative typed
heap budget. Encoding uses fallible reserves and the same item charges.

The native reader calls Reset (`0x2b6b890`), initializes definition-backed arrays,
resets the accumulated IndexList, overlays hash-resolved values and clamps
negative reserved counts to zero (`0x2b6b5c1`–`0x2b6b5cc`). Native unknown-hash
consumption, archive edits and definition remapping affect initialized objects.
This codec preserves original wire values and implements none of those live
state effects. Historical versions are rejected. No callback or gameplay
behavior is asserted.

Run the read-only audit:

```sh
python3 tools/linux/player_resources_audit.py --output evidence/linux/player-resources.json
```

The audit hash-gates both canonical Core streams and their owner traces. Traces
locate validation cuts only; the production parser receives just record bytes.
Every saved Resources owner record must decode and re-encode byte exactly under
exact byte/item caps; one-short caps must fail. Isolated tests cover nonempty
nested records, duplicate hashes, signed extremes, every truncated prefix,
trailing bytes, version/bool rejection and hostile counts. The report records
source and executable identities. Original game code is never executed, and
private artifacts are never modified. These existing saved records do not
provide a new native mutation oracle or complete nonempty gameplay coverage.

Discovery attribution: pinned `cru121` revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`; Linux symbols, DWARF, bytes and
saved records supply acceptance evidence. No proprietary code or assets are
included. Complete Player loading, world import, gameplay state, actions and
turns remain unimplemented. The recorded raw Save→Load difference remains
89,198 bytes; unknown bytes are unclassified and unmasked.
