# Linux Player.Quests saved wire

`player_quests::Quests` implements the complete current version 1 saved record:
version i32 (bits `1`) followed by owning PlayerTypes raw signed i32 ordinal.
The record is exactly eight little-endian bytes. No quest parameter records,
collections, bools, strings, hashes, Q8 values or nested versions occur here.
No arbitrary ordinal is rejected or remapped. The null parent writer uses `-1`.

The pinned Linux writer is `0x2b5cb40` (146 bytes), reader `0x2b5cbe0`
(68 bytes), and version word `0x2f665b0`. Player calls the writer at
`0x2b4f158` through PLT `0x1e0a540`. DWARF identifies only the 152-byte
sync archive and eight-byte parent pointer in this 160-byte native object.
Both enum serializers transit the signed int serializers with four-byte stream
requests. The next Player component is WMDs at call `0x2b4f16b`, PLT
`0x1ed0cd0`. These are target-verified Linux contracts, not inferred contracts
from shared names or historical Windows code.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode` and
`encode_with_items` use `unit::Limits`. Fixed scalars charge zero collection
items. Decode allocates no dynamic storage. Encode checks the eight-byte cap
before fallibly reserving its output. Zero item capacity is valid; a one-short
item cap is inapplicable. Unknown versions and incomplete records fail, strict
decode rejects trailing bytes, and prefix decode stops after the complete record.

The native reader consumes the version without branching, resolves the saved
ordinal via `Context::Globals::GetPlayer` and calls Initialize at `0x2b5cc30`.
Initialize assigns the parent and sets the archive ID according to participant
status. The codec deliberately supports only current wire and does not perform
those native initialization effects. It neither calls Reset nor serializes
QuestData. Quest generation, NPC outcomes and gameplay RNG remain in scope for
complete gameplay; this narrow Player carrier does not establish their absence
or a planning-only exclusion. Every player decision remains an external policy.

The reproducible read-only audit is:

```sh
python3 tools/linux/player_quests_audit.py --source-commit COMMIT \
  --output evidence/linux/player-quests.json
```

Commit the module and actual driver together before building. The audit builds
one isolated committed archive and records source and candidate identities. It
SHA-gates the exact GameCore and host, full serializer method bytes, direct and
PLT bindings, DWARF, and both canonical saved streams. Every original saved
record must roundtrip at exact byte/item caps, reject every truncated prefix
and reject a seven-byte cap. Source tests cover signed extrema, trailing data,
strict/prefix APIs and unsupported versions. Bool/count tests are inapplicable
because there are no bools or counts. Owner trace cuts are validation inputs
only; the production codec has no offsets or trace inputs. No original game
code executes, and no private artifacts are written.

Complete Player loading, Core/world import, exact gameplay state, action
execution and turns remain unimplemented. Unknown bytes stay unclassified and
unmasked, including the recorded 89,198-byte raw Save→Load difference. Saved
wire roundtrip does not establish native Read or gameplay parity.
