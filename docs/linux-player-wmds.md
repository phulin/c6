# Linux Player.WMDs saved wire

`player_wmds::Wmds` implements the complete current version 1 saved record:
version i32 (bits `1`), unsigned u32 definition count, then exactly that many
ordered pairs of definition type hash u32 and signed weapon count i32. All
fields are little endian. These are integer weapon counts, not Q8. The codec
preserves every hash, order, duplicate and count bit; it neither interprets
hashes as SQL ordinals nor remaps them. The parent pointer, sync archive and
native array-size member are not additional serialized fields. There are no
bools, strings, fixed arrays or nested version records in this grammar.

The target Linux writer is `0x2bc5fb0` (67 bytes), reader `0x2bc6000`
(73 bytes), current-version word `0x2f6b014`. The Player writer calls WMDs
at `0x2b4f16b` through PLT `0x1ed0cd0`. Array helpers are `0x2bc6480`
(141 bytes, WriteIndexHashArray) and `0x2bc6510` (149 bytes,
ReadIndexHashArray). The writer obtains the active WMD definition count,
iterates definitions in their native order, emits each definition's type hash,
then the corresponding integer count. Scalar serializer bodies confirm four-byte
requests. DWARF identifies a 224-byte native object containing the sync archive,
parent pointer, integer vector and array-size member. The audit verifies full
method bytes, scalar request construction and every direct/PLT binding within
the recorded methods against the exact Linux target. No historical Windows
contract establishes support here.

Native Read consumes the version without branching, edits the existing synced
vector, resolves hashes against the active WMD definitions and stores recognized
counts. Unrecognized hashes still consume their signed count into a temporary;
duplicate recognized hashes therefore overwrite earlier values. Read does not
call Reset or Initialize. The saved-wire codec deliberately preserves the wire
sequence and does not reproduce definition remapping, existing-array mutation
or synchronization effects. Weapon state, NPC state and gameplay RNG remain in
scope; no planning-only exclusion follows from a namespace. Weapon deployment
and every other player decision remain external policy decisions.

The strict and prefix APIs are `decode`, `decode_prefix`,
`decode_prefix_with_items`, `encode` and `encode_with_items`, using `unit::Limits`.
One ordered hash/count pair charges one item; record size is `8 + 8 * items`.
Decode checks the unsigned count, item cap, checked complete wire extent and
complete available payload before allocation, then checks typed allocation size
and reserves fallibly. Encode checks representable u32 count, item cap and
complete output extent before reserving fallibly. High-bit counts are unsigned
and bounded, never treated as negative signed counts. No signed collection
counts occur here. Unsupported versions, incomplete records and exceeded caps
fail explicitly; strict decode rejects trailing bytes. No trace boundary, magic
fixture offset, native owner cut or opaque skip participates in parsing.

Reproduce the read-only audit after committing both module and actual driver:

```sh
python3 tools/linux/player_wmds_audit.py --source-commit COMMIT \
  --output evidence/linux/player-wmds.json
```

The audit SHA-gates GameCore and host, verifies target contracts and DWARF, then
builds one isolated committed archive. It records archive, complete Rust source
manifest, driver, harness and candidate identities. Both canonical native streams
must roundtrip every original WMDs owner record at exact byte/item caps, reject
every truncated prefix, and reject one-short caps. The source tests exercise
nonempty duplicate hashes, signed extrema, empty records, trailing data,
strict/prefix APIs, unsigned count limits and unsupported versions. Bool tests
are inapplicable because the grammar contains no bools. Original game code is
never executed, and private artifacts are read only.

The next actual Player component is Improvements, call `0x2b4f17e`, PLT
`0x1f74d10`. Complete Player, Core/world import, exact gameplay state, actions
and turns remain unimplemented. Unknown bytes stay unclassified and unmasked,
including the recorded 89,198-byte raw Save→Load failure. Saved wire roundtrip
does not establish native live serialization, Read effects or gameplay parity.
