# Linux Player.Espionage wire boundary

[Codec](../crates/c6-core/src/player_espionage.rs), [audit](../tools/linux/player_espionage_audit.py), and [report](../evidence/linux/player-espionage.json) cover the complete current Linux Player.Espionage wire record and all six nested records. This is transport, without live definition remapping, callbacks, initialization, gameplay state, actions or turns. Full Player and world import remain incomplete.

The audit gates Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. The complete original Espionage writer is `0x2af04c0` (186 bytes), reader `0x2af0580` (377 bytes). Its component-local `Serialization::CurrentVersion` at `0x2f5dfe0` is **17**, independently verified from target bytes. The neighboring Culture version is unrelated. The audit pins 171 concrete method symbols/sizes/byte hashes, matching full DWARF types, and important direct call sites. No original game execution or proprietary project writes occurred.

The pinned cru121 revision `5af389445118bed2bdebf7a8a42b219c8fba5175` is discovery-only material under CC BY 4.0; see [attribution](reference.md). Fields and types were established from this Linux target's DWARF, ordering and overload selection from its own writer and reader bytes. Windows contracts do not establish Linux behavior.

## Complete current grammar

Every record below begins with a little-endian signed version word containing 17. Scalars are 32-bit little-endian words. Counted vectors use one unsigned 32-bit length and ordered elements, preserving duplicates. Every nested promotion list contains serialized definition hashes. No bools, Q8 values, pointers, allocator words or fixed arrays occur in this particular wire grammar.

| Record | Fields in actual wire order, following version |
| --- | --- |
| Espionage | spy capacity; CapturedSpy vector; OffMapSpy vector; CityWithSources vector; global offense bonus; global defense bonus; extra steal-tech boosts; MissionHistoryRecord vector; CounterSpyYieldAward vector; TargetCitySpyYieldPercent vector |
| CapturedSpy | owning Player raw ordinal; Unit type hash; name index; level; single promotion hash; promotion-hash vector |
| OffMapSpy | Unit type hash; name index; level; experience points; return turn; return X; return Y; return-city-owner Player raw ordinal; AI controller raw unsigned word; promotion-hash vector; old Unit ID |
| CityWithSources | Player raw ordinal; City ID; expire turn |
| MissionHistoryRecord | name index; UnitOperation definition hash; target City ID; target Player raw ordinal; target plot index; completion turn; initial result raw ordinal; escape result raw ordinal; level after mission; loot info |
| CounterSpyYieldAward | Yield raw ordinal; amount; per-X-level |
| TargetCitySpyYieldPercent | Yield raw ordinal; percent |

Hashes and IDs retain all unsigned bits, including unknown values and sentinels. All remaining scalar fields are signed integers or raw signed enum ordinals. CapturedSpy's single promotion and promotion vector are independent serialized fields. OffMapSpy serializes its AI controller after return-city ownership, despite its earlier native memory offset. Its controller word is preserved; no planning-only exclusion is inferred. MissionHistoryRecord explicitly calls `WriteIndexToHash<UnitOperation>`, whereas espionage results use raw `EnumToStream`. Yield's concrete overload writes a raw ordinal; promotion's concrete overload writes a definition hash.

Native readers accept earlier versions through field-specific compatibility logic. This codec supports only the complete current writer grammar and rejects other top-level or nested versions. Native hash-to-index lookup, old-version defaults and object/archive state reconstruction are not implemented. Plot coordinates are two signed words, without their own version.

## Bounds and validation

`Espionage::{decode,decode_prefix,decode_prefix_with_items,encode,encode_with_items}` use `unit::Limits`. Strict decode rejects trailing bytes; prefix decode reports actual consumption, and item variants also return the aggregate nested element charge. Production parsing uses only the grammar and bytes presented by the caller, with no owner trace, offset, search, default substitution or opaque skip.

Every vector checks aggregate items, minimum input and byte budgets, checks aggregate decoded heap element storage, then uses fallible reservations. Counts are charged once per collection element, including nested promotions. Byte budgets constrain consumed input/output and aggregate decoded element storage separately; decoded storage can make a valid wire record exceed a chosen budget. Unsupported input never mutates external state. There are no bool validation cases because the target grammar has no bool fields.

Run:

```sh
python3 tools/linux/player_espionage_audit.py --output evidence/linux/player-espionage.json
```

Six isolated source-only tests cover every nested type with nonempty carriers, repeated hashes, signed extremes, raw controller/ID words, roundtrip/prefix/trailing handling, every byte truncation, every nested version, exact byte/item budgets with one-short rejection, an independent complete source-carrier word fixture, and oversized/truncated allocation preflight. The harness compiles with `-Dwarnings` before exports are integrated by the parent owner.

The offline audit hash-gates two immutable original Core streams and their existing native owner traces. Each stream supplies **43 complete Espionage records totaling 31,380 bytes**, all fully consumed and re-encoded byte-for-byte. Both have ordered-record digest `998f13d92bfba6d9d57d48774f796187bf38663a3786f15de060cbf9c6db782e`. The observed first range `4,995,296..4,995,796` selects validation input only. These existing saved records do not constitute native execution of synthetic carriers or prove live Read equivalence.

The next actual Player component is **Religion**, writer call `0x2b4f09a` after Espionage at `0x2b4f087`. Complete gameplay state and RNG, all external player decisions, actions, upkeep and turns remain unimplemented. The recorded raw Save→Load difference remains **89,198 bytes**; unknown bytes remain unclassified and unmasked.
