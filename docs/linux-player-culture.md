# Linux Player.Culture wire boundary

[Codec](../crates/c6-core/src/player_culture.rs), [reproducible audit](../tools/linux/player_culture_audit.py), and [report](../evidence/linux/player-culture.json) cover the complete current Player.Culture **wire record**, including all nested records. They do not implement live Culture state reconstruction, initialized games, player decisions, action execution, upkeep or turns. Full Player and world import remain incomplete.

The audit gates original Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. The complete original writer is `0x2a6e330` (1,357 bytes), reader `0x2a6e880` (3,414 bytes). Their current version is **45**, from `Serialization::CurrentVersion` at `0x2f57f5c`. Complete target methods, concrete overloads, array helpers and matching full DWARF type identities are pinned by the audit (183 method hashes). Static original inspection and existing saved bytes were used; no original execution, private project writes, uploads or raw disassembly archives were performed.

The pinned cru121 revision `5af389445118bed2bdebf7a8a42b219c8fba5175` remains discovery-only material under CC BY 4.0; see [attribution](reference.md). Field identities/types were read from matching target DWARF and ordering from target writer/reader bytes. No cross-platform layout equality is assumed.

## Wire grammar

All integers are little-endian 32-bit words. `*_hash` fields and definition-index pair keys preserve the serialized hash, including unknown hashes and sentinels. `*_ordinal` fields preserve the signed raw ordinal. `*_bits` fields preserve signed `FixedPointT<8>` storage without conversion or rounding. All bools are canonical one-byte `0` or `1`. Every counted collection preserves order and duplicates.

The `Culture` declaration lists all 63 top-level fields in actual writer order. In sequence these are: government identity/scalars; government-unlocked bools and usage counts; last-government-per-tier hashes; five policy bool arrays; active/modified/removed government slots; acquired civics, boosts and Q8 progress; the ordered civic queue; culture overflow/rate and completion; policy-change/era-boost scalars; tourism/dominance arrays and modifiers; lifetime culture and tourist counts; tourism locations; culture-bomb/favor/feature records; great-work/favor modifiers; intolerance; previous-government policies; repeated-civic/theming/great-work scalars; slot replacements and government-type slot modifiers.

Government-unlocked, five policy arrays, acquired-civic and boost arrays reuse `player_prerequisites::IndexedBools`. Each is `u32 count; (u32 definition_hash, bool value)[count]`. Government usage and Q8 civic progress use the analogous signed-word values; last government used is `(GovernmentTier hash, Government hash)`, **not a government ordinal**. The civic queue is `u32 count; Civic hash[count]`; native inline capacity four is not a wire limit. Types::Vector and eastl::vector are count-prefixed ordered element sequences.

The four government-slot lists have an additional explicit signed count before `WriteArray`'s unsigned count. Current original writers emit equal nonnegative counts; this codec rejects mismatches. `GovernmentSlots.entries` retains the actual ordered elements and encoding emits both counts. This differs from ordinary one-count vectors.

| Nested record | Current wire fields, in order |
| --- | --- |
| GovernmentSlot | Policy hash; GovernmentSlot raw ordinal (no version) |
| GovernmentSlotReplacement | v45; replaced raw slot ordinal; added raw slot ordinal; replaces-all bool |
| GovernmentTypeSlotModifier | v45; Government hash; raw slot ordinal; signed delta |
| TouristCount | raw Player ordinal; signed count (no version) |
| TourismLocation | signed plot index; lifetime-tourism integer; counted TouristCount vector (no version) |
| CultureBombTrigger | v45; Building hash; District hash; Improvement hash; captures-owned-territory bool |
| TourismIntoFavor | v45; signed tourism; signed favor |
| FeatureUnlock | v45; Feature hash; Civic hash |
| GreatWorkObjectTourismModifier | v45; **raw GreatWorkObject ordinal**; signed tourism modifier; no-tourism bool |
| PolicyFavor | v45; Policy hash; favor per turn |
| BuildingFavor | v45; Building hash; favor per turn |

Despite its native name `m_bAlwaysFullReligiousTourism`, that member is an `FAutoVariable<int>` and occupies one signed word on the wire. GovernmentSlot's wire policy-before-slot order reverses its in-memory member order. No object padding, pointers or owner/archive structures are serialized by these records.

## Bounded APIs and live-state distinction

`Culture::{decode,decode_prefix,decode_prefix_with_items,encode,encode_with_items}` use `unit::Limits`. Prefix APIs return consumed bytes and the aggregate nested item charge; strict decoding rejects trailing bytes. Decoding bounds consumed wire bytes by `max_bytes`, checks aggregate item counts and minimum required input, and uses fallible reservations. Each individual vector allocation is checked against `max_bytes` before reservation, matching the existing `unit::Limits` policy. Typed heap storage is not accumulated against the wire-byte budget. No child collection receives a fresh independent budget. Redundant government-slot counts describe one element collection and charge its elements once. Limits may reject otherwise well-formed records if an individual decoded vector allocation exceeds the byte budget.

Only current versions are accepted, including each nested version tag. Truncation, noncanonical bools, unsupported versions, mismatched redundant counts and resource overflows are errors. Unsupported input never mutates external state. There is no unframed opaque skip, trace lookup, hard-coded owner offset or default substitution in production parsing.

Native `Read` first invokes `Culture::Reset`, overlays definition-indexed arrays into definition-sized destinations, maps hashes to current definitions and executes version/content-dependent compatibility behavior. The codec preserves the writer's bytes and does not apply those runtime operations. The located acquired-civic field is `Culture.has_civic`: original writer call `0x2a6e57a`, object value offset `0x360` (DWARF owner member `m_abHasCivic` at `0x350`); matching reader call `0x2a6ec98`. Callers may separately use the existing `IndexedBools::acquired_after_owner_read` with actual Civic definitions to model the proven Reset(false)+overlay projection. This does not establish full Culture native Read or gameplay parity.

## Checks and remaining boundary

Run:

```sh
python3 tools/linux/player_culture_audit.py --output evidence/linux/player-culture.json
```

The isolated harness exercises seven source-only tests: nonempty roundtrip/prefix/trailing handling; every byte truncation; top/nested versions; strict top/nested/indexed bools and redundant counts; exact aggregate byte/item budgets; allocation preflight; and independent vector allocations whose aggregate typed storage exceeds their combined wire length. Every collection and nested type has nonempty synthetic coverage, including duplicate hashes, signed extremes, raw ordinals and nonintegral Q8 bit patterns.

The offline audit independently hash-gates both canonical original Core streams and existing owner traces. Each stream contains **43 complete Culture records totaling 277,309 bytes**, all decoded, fully consumed and re-encoded byte-for-byte. Every record also decodes and encodes under exact serialized-byte and aggregate-item caps; reducing either cap by one is rejected. Both sets have the same ordered record digest `eac20876eb7a8ad871272f03c59a320c29d72a09d7376433ffc749e679b74f79`. Trace cuts select validation inputs only. The first observed range `4,988,394..4,995,296` is not a parser constant. These saved records do not replace nonempty source-derived fixtures or constitute native execution of synthetic cases.

The actual Player writer calls Culture at `0x2b4f074`, then **Espionage** at `0x2b4f087`; Espionage is the next unsupported owner. Complete Player/world import, actual gameplay state, gameplay RNG, all policy decisions, actions and turns remain unimplemented. The recorded raw Save→Load difference remains **89,198 bytes**, with unknown bytes unclassified and unmasked. No planning-only exclusion is inferred from namespace or ownership.
