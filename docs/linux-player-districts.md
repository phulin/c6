# Linux District and Player.Districts wire records

`player_districts::{District,Districts}` closes the current District v15 record and Player.Districts v8 envelope. These are bounded wire codecs, not live native `Read`, complete Player state, initialized world import, gameplay, actions or turns. Existing supported variant forms remain the limit: unsupported property payloads fail explicitly. Native definition remapping and omitted live-field reconstruction are not performed.

The [audit](../tools/linux/player_districts_audit.py) and [report](../evidence/linux/player-districts.json) gate Linux XP2 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. They pin complete writer/reader, overload and wrapper hashes, version constants, matching target DWARF identities and inherited property bindings. No original code was executed; no private files, projects, uploads or raw disassembly archives were written.

Field names and types come from exact-target DWARF; current order and concrete overloads come from the pinned Linux methods. Existing [District identity evidence](linux-city-district-state.md) was reused. The pinned cru121 reference revision `5af389445118bed2bdebf7a8a42b219c8fba5175` remains discovery material under CC BY 4.0; see [attribution](reference.md). No reference layout or SQL ordinal was substituted for the original Linux writer.

## District v15

Writer `0x291cd70` (487 bytes) and reader `0x291cf60` (1,103 bytes) establish this complete current order. Words are little-endian; bools are one byte and accepted only as 0/1. Every listed hash is retained as u32. Ordered hash arrays are `u32 count`, followed by `(u32 definition_hash, i32 value)` pairs, preserving duplicates and order.

| Order | Fields and framing |
|---|---|
| 1 | i32 version 15; u32 full ID; i32 location x/y; u32 parent City ID; District definition hash |
| 2 | i32 garrison damage, outer defenses damage, last turn damaged, remaining attacks, cost; completion bool |
| 3 | TypedVariantMap properties |
| 4 | Three Yield hash arrays: yield changes, base yield changes, yield adjacency modifiers |
| 5 | i32 extra regional range, extra regional entertainment, extra entertainment |
| 6 | Two Yield hash arrays: extra regional yields, tourism adjacency yield modifier |
| 7 | i32 tourism change; counted AdjacencyBonusMirror vector |
| 8 | GreatPersonClass hash array, with signed raw Q8 values |
| 9 | i32 GreatPerson point multiplier, point adder, air slots modifier, within-one-hex espionage defense bonus; under siege bool; pillaged bool |

Mirror writer/reader `0x291cc10`/`0x291cc70` use **version 15**, two explicit Yield index-to-hash conversions and signed mirror count. These are hashes even though the scalar `YieldTypes` overload used elsewhere writes a raw index. GreatPerson fixed points retain all signed 32-bit bits without floating-point conversion.

District's property value subobject at live `+0x380` calls writer virtual slot `+0x1f8` with flag 1, and reader slot `+0x1f0` with flag 0. The constructed host TypedVariantMap table `0x53b3680` binds these to `0x3961e50` and `0x3971330`, respectively, as for City properties. This is the typed map, not the untyped VariantMap used by some other components.

`m_aAppealYieldChanges`, deletion marker, combat parent pointer, archive bookkeeping and other runtime storage are absent from this writer. Their absence is not a gameplay exclusion or a promise that a loaded live object is ready to execute. Native `Read` finally derives its archive ID through `GetComponentID`; this wire codec does not emulate that live fixup.

There is no serialized District owner field. Actual ownership remains District.GetPlot → Plot.GetOwner. The enclosing Manager/table player is separate context and must not be copied into an actual District-owner observation. Full instance ID, parent City ID, plot location, saved definition hash and runtime definition index remain distinct.

## Player.Districts v8

Writer `0x2adff90` (325 bytes), reader `0x2ae00e0` (762 bytes), and the concrete recycled writer/reader `0x2ae3000`/`0x2ae3240` give:

1. i32 version 8.
2. Recycled container: u32 version **65537**, u32 index bits **16**, u32 capacity, u32 used count; repeated `(i32 explicit slot, complete District)`; u32 current ID bits, u32 free count, u32 free head, u32 free tail; exactly `free count` u32 links. There is **no additional count** before links.
3. i32 free-building-when-specialty-district-constructed count; i32 non-specialty envoys.
4. Four District hash arrays: current cost, specialty free-building exceptions, specific-district envoys, cannot-build-adjacent-to-city counts.
5. One Feature hash array: specialty-district feature requirements.
6. Five counted vectors with versioned elements, in the following order.

| Element | Current version | Exact payload |
|---|---|---|
| DistrictCreateYield | 1 | District hash; **raw signed Yield index**; i32 amount; appeal bool; must-replace-improvement bool; Improvement hash |
| DistrictCreateUnit | 1 | District hash; u32 Unit type hash |
| DistrictAndBuildingsCreateUnitWithAbilityByClass | 1 | UnitAbility hash; District hash; UnitPromotionClass hash |
| DistrictUnlockTech | 1 | District hash; Technology hash |
| DistrictUnlockCivic | 1 | District hash; Civic hash |

The first scalar Yield overload is `0x2d4fb70`, whose stream scalar is the original signed enum word. It must not be changed to a Yield definition hash merely because other neighboring overloads remap definitions. Nested writers/readers span `0x2adfc90`–`0x2adff89`. The runtime `m_districtBuildMap` is absent from this writer. Native `Read` includes definition-sized allocation/default initialization and rebuild work; no equivalent is claimed by preserving the bytes.

Both codecs expose strict `decode`/`encode`, plus composable `decode_prefix_with_items` and `encode_with_items` under `unit::Limits`. Prefix decoding reports consumed bytes and aggregate dynamic items; the caller carries the remaining budget forward. Counts across properties, arrays, records, links and vectors share one item budget. Fallible allocation and byte limits bound owned vectors. Invalid versions, noncanonical bools, truncated records, duplicate/out-of-range explicit slots and exceeded limits fail. Strict decoding also rejects trailing bytes. No partial state is returned on failure.

Allocator entry order, full IDs, wrapping counter bits, head/tail/link values and unused link sentinels are preserved. Slot validation does not invent extra generation comparisons or silently canonicalize the table. The codecs neither create nor remove Districts.

## Validation and limits

Run `python3 tools/linux/player_districts_audit.py --output target/linux/player-districts.json`. The audit builds an isolated Rust harness around the owned module; it does not require a composition/export change or native execution. Sources and candidate binary are fingerprinted. Existing canonical Core streams and immutable owner traces are hash gated; traces define validation cuts only and are not available to the production decoder.

Each of two original normalized streams supplies **1,284 full District records (543,132 bytes)** and **43 full Player.Districts envelopes (602,500 bytes)**. Every cut decodes to its original end and reencodes byte-identically, with matching decode/encode aggregate item usage. Independent fixed-prefix checks verify all District identity/location/city/type/completion values. Both streams have identical District and envelope record digests. This checks offline original serializer outputs, not dynamic execution or live Read→Save equivalence.

Two focused Rust tests use nonempty, distinct source carriers across every serialized nested vector, named Char8 properties, duplicate/unknown hash entries, negative Q8 values, nonzero full IDs/counters, out-of-order explicit slots and free links. They test every truncation of both complete carriers; all nested version and bool boundaries; malformed count/slot/container metadata; strict trailing rejection; and exact shared byte/item limits in decoding and encoding. Existing variant transport retains its documented unsupported branches.

Complete Player roster parsing and following components remain separately composed work. Complete gameplay state and RNG, exact action execution, rules/upkeep between external player decisions and turns remain unimplemented. The recorded **89,198-byte raw Save→Load difference** remains unmasked and unclassified; these wire records do not justify excluding any AI-owned or unknown state.
