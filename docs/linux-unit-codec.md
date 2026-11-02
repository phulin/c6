# Linux Unit wire components

The Rust library still implements isolated primitives. `c6_core::unit` now decodes and encodes the **complete current Formation version-1 wire record**, and a bounded operation-queue component. It does not load a complete Unit or Core Save, reconstruct live native objects, implement SKIP_TURN, or establish state/action/turn equivalence. The original Linux XP2 module is pinned to SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.

The [original Unit/Formation contract](linux-unit-archive.md) and [schema audit](../evidence/linux/unit-archive-schema.json) establish the actual serializer helpers independently from the Rust implementation. Public evidence contains identities, counts and checks; proprietary payloads and raw disassembly remain private.

## Formation wire support

Formation records preserve unsigned IDs, signed 32-bit player/domain enum bits, signed 32-bit PlotCoord coordinates, raw `FixedPointT<8>` values, eight individual bool bytes, original path order, signed 16-bit path-node coordinates, and ordered uint32 unit IDs. No sorting, floating-point conversion, definition remapping or inferred unit membership occurs. Duplicate unit IDs are retained as wire values. The native writer emits version 1, path count and ordered version-1 nodes, cache coordinates, then unit count and ordered IDs. The record size is `52 + 20 * path_nodes + 4 * unit_ids` bytes.

`Formation::decode` consumes the exact provided record. Unsupported record/node versions, trailing or truncated bytes, noncanonical bool bytes and resource-budget violations return errors. Version and bool restrictions describe this supported codec profile; they do not claim the original reader rejects those inputs. `Limits` bounds bytes and the aggregate number of path nodes plus unit IDs before allocations. Encoding applies the same budgets.

Run the original-record comparison without executing the engine:

```sh
python3 tools/linux/unit_codec_check.py --output target/linux/formation-rust-codec.json
```

The checker gates two exact original Save hashes and their exact native serializer trace hashes, checks the original module identity in the private canonical-stream manifest, and extracts **only original `GameCore::Formation::Instance` owner ranges**. It never infers record boundaries from ID patterns. Every decoded record must consume its exact range, reproduce its bytes, and match its typed predicted length. The [checked result](../evidence/linux/formation-rust-codec.json) covers 1,250 formations per stream, 2,500 total round trips. Each stream contributes 71,168 bytes, 56 path nodes across eight nonempty paths, and 1,262 ordered unit IDs. Formation bytes happen to be identical across this pair; this does not establish equality of other state.

Constructed independent wire fixtures additionally cover signed/unsigned scalar extremes, raw enum and fixedpoint values, packed signed shorts, invalid cache coordinates, duplicate/ordered IDs, empty containers, all truncated prefixes, unsupported versions, malformed bools, malicious counts, trailing bytes and exact aggregate resource limits. Release timings in the checked report measure only actual Formation record decode and encode, including allocation/drop. They exclude game initialization, rules, actions, turns, RNG and original-code performance.

The actual native Formation reader also resets the omitted evaluating-operation flag, resolves the original owner pointer, and sets archive identity. Those live effects are not encoded in this Rust wire model. Archive dirty-variable history, original pointer/registration identity and full live state remain separate requirements.

## Bounded operation queue

`OperationQueue` supports current queue version 1 and base node version 2. Nodes preserve the operation **hash**, directive, flags, push turn, saved activity and saved automation. Live numeric handler IDs are distinct. Supported handlers are SkipTurn, Alert, Fortify, SpySiphonFunds, SpyDisruptRocketry, SpyGainSources, SpySabotageProduction, MoveTo and MakeTradeRoute. The [original handler contract](../evidence/linux/unit-wire-contracts-operations.json) identifies each target hash, inherited serializer, parameter factory and exact current parameter layout.

The seven null-parameter handlers use sentinel 0. MoveTo has version-1 parameters containing PlotCoord then uint32 modifiers; MakeTradeRoute has version-1 parameters containing uint32 modifiers, two PlotCoords and the resource definition hash. `OperationParameters` distinguishes these forms. Encoding rejects a parameter variant that does not match its handler; decoding rejects unknown handlers and unsupported versions. There is no generic handler-body length permitting opaque skipping. Historical versions, other handlers and live definition remapping remain unsupported.

The pinned native traces attribute 1,262 queue records per stream. The checker now roundtrips **all 2,524 queue records**: per stream 954 empty, 185 MakeTradeRoute, 91 MoveTo, and 32 with one of the six observed null-parameter handlers. All exact native queue bounds are consumed and bytes reproduced. Independent constructed fixtures additionally cover multi-node order, signed field extremes, parameter modifiers/resource hashes, handler/parameter mismatches, malformed parameter versions and every truncated prefix. No actual SkipTurn queue appears in these inputs, so its nonempty native fixture remains a validation gap. Serialized queue parity does not implement any handler's gameplay behavior or establish complete Unit/action parity.

## Pending full Unit support

The current Unit version-28 top-level order is known, but its nested definition-indexed arrays and maps, components, Hero, property variants and all operation-handler/parameter formats still need complete contracts and attributed fixture validation. The codec cannot safely advance past those unknown nested fields. Full Unit decoding and definition remapping remain pending. Gameplay actions remain unsupported and cannot advance state through these wire records. The complete exact simulator objective remains open; all player decisions must yield to external policies.

The raw Save→Load→Save failure remains **89,198 bytes**, unclassified and unmasked. These successful component round trips neither repair nor normalize that failure.
