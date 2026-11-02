# Linux Unit and Formation wire codecs

The Rust library still implements isolated primitives. `c6_core::unit` now decodes and encodes **current Unit version-28 and Formation version-1 wire records**, with explicit gates for supported operation handlers and typed variants. It does not load a Core Save, reconstruct complete live objects, implement SKIP_TURN, or establish state/action/turn equivalence. The original Linux XP2 module is pinned to SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.

The [original archive contract](linux-unit-archive.md), [named component sequences](../evidence/linux/unit-wire-contracts.json), [scalar helpers](linux-unit-wire-scalars.md), [operation helpers](linux-unit-wire-operations.md) and [variant helpers](linux-unit-wire-variants.md) ground the Rust code in original Linux instructions and matching DWARF. Public evidence contains identities and checks; proprietary payloads and raw disassembly remain private.

## Unit support

`Unit` has all 73 current serialized fields in the original order, including unregistered Hero, attributes and properties. Eighteen named nested component types retain their current versioned fields. Definition enums retain uint32 hashes; plain enums retain signed 32-bit bits. Definition-indexed arrays preserve every serialized hash/value pair in order, including duplicates. Promotion maps preserve uint8 values without treating them as bools. Fixedpoint values remain raw int32; strings remain counted bytes without a UTF-8 assumption. City IDs preserve signed 16-bit owner/component halves and uint32 identity. No sorting or live definition remapping occurs.

`Unit::decode` consumes exactly one supplied record. `decode_with_ranges` additionally exposes each field's consumed range for comparison with original serializer traces. Both reject unsupported nested versions, handlers, variants, malformed bools, truncated/trailing bytes and resource-budget violations. Encoding validates the same supported shape. `Limits` bounds total bytes and aggregate dynamic elements/string bytes before allocations; recursive variants additionally have a maximum depth of 64. Restrictions describe the supported Rust profile, rather than claiming the original reader rejects every such input.

`VariantMap` and `TypedVariantMap` preserve unsigned keys, original ordered tree/vector entries, optional key names, prefix metadata, scalar union bits and established recursive framing. Supported semantics include scalar None/bool/int32/uint32/float32/char8/int64/uint64 and supported nested containers. Bool upper union bytes and float NaN bits are retained. Unknown semantic types, arbitrary raw arrays, unsupported modifiers/branches and unproven payload forms reject explicitly. The [variant module](../crates/c6-core/src/unit/variants.rs) states the detailed accepted subset. A known payload framing alone does not establish every possible variant's gameplay meaning.

## Formation support

Formation records preserve unsigned IDs, signed 32-bit player/domain bits and PlotCoord coordinates, raw fixedpoint values, eight bool bytes, ordered path nodes with signed 16-bit coordinates, cache location, and ordered unit IDs. The native record size is `52 + 20 * path_nodes + 4 * unit_ids`. No inferred membership or pointer reconstruction occurs.

## Operation queue support

Queues use current version 1 and base nodes version 2. Nodes preserve the operation **hash**, directive, flags, push turn, saved activity and saved automation. Live numeric handler IDs are distinct. Supported handlers are SkipTurn, Alert, Fortify, SpySiphonFunds, SpyDisruptRocketry, SpyGainSources, SpySabotageProduction, MoveTo and MakeTradeRoute.

Seven null-parameter handlers use sentinel 0. MoveTo version-1 parameters contain PlotCoord then uint32 modifiers. MakeTradeRoute version-1 parameters contain modifiers, two PlotCoords and a resource definition hash. `OperationParameters` distinguishes these forms; encoding rejects mismatched handler/parameter variants. Unknown handlers reject because there is no generic handler-body length permitting opaque skipping. Broader static handler research is not counted as implemented support.

## Native record validation

Run the read-only checker, which never executes the engine:

```sh
python3 tools/linux/unit_codec_check.py --output target/linux/unit-rust-codec.json
```

The checker gates two exact original Save hashes and exact native serializer trace hashes, and checks the original module identity in the private canonical-stream manifest. It extracts only original serializer-owned ranges. It never infers boundaries from ID byte patterns. The [checked result](../evidence/linux/unit-rust-codec.json) covers, across both streams:

- **2,524 complete Unit round trips**, 6,137,501 bytes per stream, with **17,668 exact nested boundary comparisons** for CombatModifiers, MovementModifiers, Experience, Religion, RockBand, Espionage and OperationQueue.
- **2,500 complete Formation round trips**, 71,168 bytes per stream, including 56 path nodes across eight nonempty paths and 1,262 ordered unit IDs per stream.
- **2,524 separate queue round trips**. Per stream: 954 empty, 185 MakeTradeRoute, 91 MoveTo, and 32 with six observed null-parameter handlers. No actual SkipTurn queue appears; its nonempty native fixture remains a gap.

The checker additionally rejects all truncated prefixes of a real Unit, malformed native nested versions/bools/counts and trailing bytes. It finds the exact aggregate-item threshold for that real Unit and verifies the same threshold in encoding; exact byte-budget boundaries also pass. Native Unit keys are unique by `(owner,id)`, and adapter lookup results reference the parsed records. Constructed independent fixtures cover scalar extremes, component ID halves, raw strings, duplicate hash pairs, nonboolean promotion bytes, handler-specific parameters, recursive variants and malformed resource/depth boundaries.

Release timings measure actual record decode and encode including allocation/drop. Unit decode includes field-range collection; lookup uses an adapter `HashMap<(owner,id),index>` and excludes building it. These timings exclude game initialization, rules, actions, turns, RNG and original-code performance.

## Remaining live state and acceptance gaps

Current wire records omit native Unit domain and Formation evaluating-operation, along with pointers and other live data. Native readers derive fields, resolve owner/definition references and set archive identity. Those effects, complete live state, definition remapping and tracked archive dirty history are not implemented by this codec. Other operation handlers, other variant types and historical versions remain explicitly unsupported. Successful current-fixture round trips do not establish universal support for all possible Unit records or gameplay behavior.

SKIP_TURN still requires its full legality, queue mutation/dirty history, formation effects, callback/events, diplomacy, notification and RNG surfaces, plus the genuine request delivery and reversible original fixture. All player decisions must yield to external policies. The full exact simulator objective remains open.

The raw Save→Load→Save failure remains **89,198 bytes**, unclassified and unmasked. These component round trips neither repair nor normalize that failure.
