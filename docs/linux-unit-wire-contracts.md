# Linux Unit nested writer contracts

The library still implements isolated primitives, not complete Civ VI state/action/turn behavior. This work recovers the current nested Unit **wire writers** for the pinned Linux XP2 module. It does not establish complete live state, native action parity, historical-reader compatibility, or a working full save loader. Raw Save→Load differences, including the recorded **89,198-byte failure**, remain unmasked.

`python3 tools/linux/unit_wire_contracts_audit.py --output target/linux/unit-wire-contracts.json` gates both supplied ELF SHA-256 identities, extracts bounded original writer calls, resolves each component member against matching DWARF, and verifies the actual host TypedVariantMap virtual slots. The [machine contract](../evidence/linux/unit-wire-contracts.json) records ordered field names, source offsets, exact serializer types, current version constants and method fingerprints for 18 nested component records. `--private-output ../c6-ghidra/evidence/linux-unit-wire-contracts` optionally preserves bounded original disassembly and DWARF privately. No game code executes.

The reference used for discovery is cru121's CC BY 4.0 [reference at the pinned revision](https://github.com/cru121/civ6-gamecore-reference/tree/5af389445118bed2bdebf7a8a42b219c8fba5175); see [reference attribution](reference.md). Matching Linux bytes and DWARF, not reference names or Windows layouts, determine these contracts.

## Current component records

Every `fields` sequence in the artifact is wire order, not object memory order. `constant_i32` identifies a signed 32-bit record version. `value_offset` locates the actual source member within the nested native object; it is not an offset in a save stream. `serializer` identifies the exact next nested contract. Fields named under `members_absent_from_this_writer` remain separately observable live members; their omission classifies neither their ownership nor gameplay significance.

| Component | Current version | Writer |
|---|---:|---|
| HealthModifiers | 2 | `0x2c241d0` |
| HealthReligionModifiers | 1 | `0x2c242a0` |
| CombatModifiers | 28 | `0x2c24370` |
| MovementModifiers | 8 | `0x2c24bd0` |
| SightModifiers | 1 | `0x2c24e30` |
| AttackRangeModifiers | 1 | `0x2c24eb0` |
| Experience | 7 | `0x2c17db0` |
| GreatPerson | 1 | `0x2c194b0` |
| Religion | 8 | `0x2c43ad0` |
| RockBand | 10 | `0x2c44af0` |
| Trade | 2 | `0x2c48500` |
| Archaeology | 4 | `0x2c11e30` |
| Espionage | 7 | `0x2c15bc0` |
| Hero | 5 | `0x2c1bb20` |

Sight::PlotEntry, EspionageBonus, EspionageTimeReduction and InitiationYieldPopulation have no per-record version prefix in their current writers. Their exact ordered fields are also in the artifact. Archaeology's fields belong to `m_kVariants`, whose matching reflected type is `GameCore::UnitArchaeology::Reflection::Variants1`; those subfield names are preserved.

A component's enum member is not automatically a raw runtime enum. For example, Hero's class passes through the real HeroClass definitions to index→hash serialization. Its current record is signed int32 version 5, uint32 HeroClass definition hash, then CityID's signed int16 owner, signed int16 component type and uint32 ID. The parent pointer and padding are absent. Older Hero readers consume obsolete fields at specific version cutoffs; accepting only the current writer version is a distinct, narrower contract.

## Definition arrays and promotion maps

The six top-level definition arrays write **every definition entry**, including false/zero values. They emit uint32 definition count, then iterate original `At(i)` in ascending index and write uint32 `GetTypeHash()` followed by the element. The ability and GreatPersonClass arrays use signed int32 values; Terrain and Feature arrays use the original bool serializer. Do not replace the count with the number of nonzero entries or treat hashes as live indexes. The same helper family supplies nested promotion and RockBand arrays; the exact per-field definitions remain part of the configuration.

The reader first consumes the saved uint32 count and then every hash/value pair. A known hash writes the resolved destination index; an unknown hash consumes the typed value into a temporary. This helper does not clear all destination elements. Duplicate known hashes overwrite in stream order. Exact live state therefore requires the original definition mapping and destination initialization, separately from a lossless wire representation.

Promotion-class `HashTable<unsigned char>` writer `0x2c2a190` emits uint32 size, then native const-iterator order as uint32 key and uint8 value. This is not definition-array serialization. Preserve observed sequence for round trips; a sorted map cannot establish the original hashtable iteration order.

## Other framing and work boundaries

Trade's `StaticArray<int,5>` writer `0x2c48940` emits exactly five signed int32 values without a count. Experience's `String::Encoding(1)` writer `0x264bed0` emits uint32 `GetLength()` and exactly that many original bytes, without an added terminator. The encoding name alone does not establish a multiplier of two or a host Unicode conversion contract. Fixed-point members invoke their original fixed-point serializer and must preserve underlying bits.

[Scalar and container contracts](linux-unit-wire-scalars.md), [operation-handler contracts](linux-unit-wire-operations.md), and [recursive variant contracts](linux-unit-wire-variants.md) track the independent remaining helpers. Formation/path ownership remains in [the Unit archive contract](linux-unit-archive.md). The component artifact supplies current writers; these helper reports provide their transitive framing and explicit unsupported cases.

Acceptance requires the Rust owner to compare typed decode/re-encode results against original attributed Unit/component slices, including nonzero and dynamic branches. Static recovery alone is not a passing native oracle. Missing configuration mappings, handler variants, live fields, or state transition dependencies cannot be advanced as opaque data or fulfilled by native AI policy.
