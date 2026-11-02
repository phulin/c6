# Linux Unit operation queue wire contracts

The implementation covers isolated primitives. This is static serializer evidence for the pinned Linux XP2 module `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and matching host, not gameplay operation execution or complete Unit/state/turn acceptance.

Run `python3 tools/linux/unit_wire_contracts_operations.py --output target/linux/unit-wire-contracts-operations.json`. The checked-in [report](../evidence/linux/unit-wire-contracts-operations.json) gates both original ELF identities, resolves actual ELF virtual-table relocations, fingerprints methods and verifies the observed handler constructors' hash assignment and virtual-table identity. Optional `--private-dir ../c6-ghidra/evidence/linux-unit-wire-contracts/operations` saves raw disassembly and matching DWARF privately. The audit does not open Ghidra or execute native game code.

Discovery consulted cru121's CC BY 4.0 [reference](reference.md), pinned at `5af389445118bed2bdebf7a8a42b219c8fba5175`. The original Linux target independently establishes the hashes, constructor assignments and serializer bindings below. Shared names and historical Windows addresses are not parity evidence.

The outer queue writer is `0x2da66b0`; its reader is `0x2da6780`. As described in [the Unit archive contract](linux-unit-archive.md), it writes int32 version 1, uint32 count, then each original queue entry in order: uint32 operation hash followed immediately by the resolved handler's node body. Hashes are distinct from runtime operation IDs. There is no generic payload length permitting unknown handlers to be skipped.

Every one of the 90 operation-related class virtual tables discovered here binds node Write at virtual offset `0xb0` to BaseHandler `0x2d965a0`, and Read at `0xb8` to `0x2d96630`. This inventory includes BaseHandler and SpyBaseHandler and expansion replacements; it is not a claim that 90 distinct operations are enabled in a configuration. The Manager table is explicitly excluded. Concrete factories, hash-return methods and registration methods are reported where present; runtime enabled registration and replacement order remain configuration-dependent.

A current node body contains int32 version 2, directive enum32, int32 flags, int32 push turn, saved activity enum32 and saved automation enum32, followed by its parameter body. BaseHandler Read selects parameters through handler virtual MakeParameters at `0x80`, then parameter virtual Read at `0x10`; the writer uses parameter virtual Write at `0x18`. Versions below 2 omit the saved fields and assign both -1 during native read. Only the evidenced current version should be accepted by a bounded current codec. For null parameters, the writer emits int32 zero. The native null reader consumes this integer without checking it; a bounded codec may reject nonzero values without claiming native corrupt-input equivalence.

The eight queue hashes observed in the attributed canonical fixture map to:

| Wire hash | Linux handler class | Parameter body | One-entry queue bytes |
|---|---|---|---:|
| `8374d954` | `XP2::MakeTradeRoute` | MakeTradeRoute ClassParameters | 64 |
| `580f2f68` | `MoveTo` | SharedParameters SingleLocation | 52 |
| `e547bc17` | `Alert` | null | 40 |
| `d3a6f7e8` | `Fortify` | null | 40 |
| `223eea54` | `XP1::SpySiphonFunds` | null | 40 |
| `c31647e8` | `XP1::SpyDisruptRocketry` | null | 40 |
| `ed72b77d` | `XP1::SpyGainSources` | null | 40 |
| `09c1ecc1` | `XP1::SpySabotageProduction` | null | 40 |

All six null handlers resolve to BaseHandler MakeParameters `0x2d96820`, which returns an empty shared pointer. Their individual GetOperationHash functions return the listed immediate constants, and constructors store them at handler `+0xc`; BaseHandler GetHash reads `+0xc`. The native numeric type lives separately at `+8`. Expansion replacements may share a wire hash with their base class: a hash alone cannot establish which gameplay implementation is active. The report explicitly groups every duplicate hash and asserts equal current parameter wire fields within each group. In particular, hash `1d60e778` has distinct BuildImprovement and XP2 BuildImprovement parameter classes, but both write version 1, uint32 modifiers and uint32 Improvement definition hash. This establishes a shared current wire shape; configuration still selects the actual gameplay class. A codec must compare duplicate entries instead of accepting the last map insertion silently.

MoveTo MakeParameters `0x2da4d30` constructs SharedParameters SingleLocation. Its writer `0x2d51ce0` and reader `0x2d51d20` use this exact sequence:

1. int32 version 1;
2. location x and y, each int32, from object `+0xc` and `+0x10`;
3. uint32 modifiers from object `+8`.

That is 16 wire bytes. The modifiers come **after** the coordinate despite their earlier object offset. Parameter object size is 24 bytes; allocator state, virtual pointers and padding are not serialized. These readers consume the version without branching on it; accepting other versions still needs evidence.

XP2 MakeTradeRoute inherits MakeTradeRoute MakeParameters `0x2da0fc0`, constructing MakeTradeRoute ClassParameters. Writer `0x2da04c0` and reader `0x2da0520` use:

1. int32 version 1;
2. uint32 modifiers from object `+8`;
3. location0 x/y int32 from `+0xc`/`+0x10`;
4. location1 x/y int32 from `+0x14`/`+0x18`;
5. uint32 Resource definition hash, translated from object ResourceTypes at `+0x1c`.

That is 28 wire bytes. Resource writer `0x2d4ee80` obtains original ResourceDefinitions and calls WriteIndexToHash; reader `0x2d4eeb0` reads uint32 then HashToIndex with those definitions. Keeping a typed `resource_definition_hash` in a wire-state model preserves the actual wire identity; assigning it directly as a live ResourceTypes enum is incorrect. Full live restoration still requires the exact Linux definitions and configuration. Coordinate helpers write two signed int32 scalars. Scalar bytes are little-endian for this pinned target.

The broader inventory resolves 29 unique MakeParameters factories into 12 concrete parameter types plus null. The report preserves each type's actual virtual writer/reader fingerprints, ordered serializer calls with source object offsets and named wire fields. Every parameter version is int32 1. The remaining current payloads are:

| Parameter type | Fields after version, in wire order |
|---|---|
| SingleUnit | target owner int16, component type int16, ID uint32, modifiers uint32 |
| BuildDistrict | modifiers uint32, eDistrictType definition hash uint32 |
| BuildImprovement / XP2 BuildImprovement / Repair / Pillage | modifiers uint32, eImprovementType definition hash uint32 |
| BuildRoute | modifiers uint32, eRouteType definition hash uint32 |
| WMDStrike | modifiers uint32, eWMDType definition hash uint32, kLocation0 x/y int32 |
| XP2 BuildImprovementAdjacent | modifiers uint32, eImprovementType definition hash uint32, kLocation0 x/y int32 |
| ExecuteScript | modifiers uint32, kParameters TypedVariantMap serialized with options 1 |

Player UnitID writer `0x2c9adc0` serializes original ComponentID getter outputs as owner int16, type int16, ID uint32; reader `0x2c9ae20` reads those scalars then invokes ComponentID Set. This is eight wire bytes. Definition enum writers/readers are independently fingerprinted and checked for original GetDefinitions plus WriteIndexToHash / HashToIndex call chains. They preserve uint32 definition hashes in the wire model. The ExecuteScript TypedVariantMap contract is coordinated with [the nested Unit wire report](../evidence/linux/unit-wire-contracts.json). Its variant/index context remains necessary; preserving raw object bytes would not establish its semantics. Matching original DWARF parameter field names are retained in the private evidence.

No native run or native oracle comparison is performed by this audit. Attributed fixture parsing and byte comparisons belong to `tools/linux/unit_codec_check.py` and its own evidence report. This static contract does not establish legality, action effects, lifecycle subscriptions, RNG effects, queue continuation or exact turns. Unknown operation hashes, unsupported parameter types and unsupported versions must be rejected without mutation by an implementing codec. The recorded raw Save→Load **89,198-byte difference** remains unclassified and unmasked.
