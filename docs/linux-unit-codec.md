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

`OperationQueue` supports current queue version 1, ordered nodes with the **SkipTurn operation hash** `0x2c681908`, current base node version 2, raw directive/flags/push-turn/pre-operation activity/automation words, and the absent-parameter sentinel 0. Live numeric handler IDs are not wire hashes. Unknown handlers and nonzero parameter versions fail explicitly: handler bodies have no shared length that would permit opaque skipping. Historical queue/node versions are unsupported.

The queue component is supported by original-code static contracts and independent constructed boundary tests. **No attributed native queue record has been compared**, so native queue acceptance remains a validation gap. It must not substitute for complete Unit state or action parity.

## Pending full Unit support

The current Unit version-28 top-level order is known, but its nested definition-indexed arrays and maps, components, Hero, property variants and all operation-handler/parameter formats still need complete contracts and attributed fixture validation. The codec cannot safely advance past those unknown nested fields. Full Unit decoding and definition remapping remain pending. Gameplay actions remain unsupported and cannot advance state through these wire records. The complete exact simulator objective remains open; all player decisions must yield to external policies.

The raw Save→Load→Save failure remains **89,198 bytes**, unclassified and unmasked. These successful component round trips neither repair nor normalize that failure.
