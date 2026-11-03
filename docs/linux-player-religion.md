# Linux Player.Religion wire boundary

The [codec](../crates/c6-core/src/player_religion.rs), [audit](../tools/linux/player_religion_audit.py) and [report](../evidence/linux/player-religion.json) cover the complete current **Player.Religion v20 wire grammar**, distinct from Game.Religion and City.Religion. This is bounded transport. Complete Player/world loading, live object reconstruction, gameplay state, actions and turns remain unimplemented.

The audit requires original Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. The full writer is `0x2b5d1a0` (549 bytes), reader `0x2b5d3d0` (1,499 bytes). Component-local `Serialization::CurrentVersion` at `0x2f66664` contains **20**. The audit pins full bytes, symbol addresses and sizes of 63 transitive stream methods, complete matching DWARF types, and concrete call/jump bindings including PLT relocation targets. Player's call `0x2b4f09a` targets PLT `0x1f8ff50`, whose relocation resolves to the actual Religion writer; the PLT is not the writer body.

The pinned cru121 revision `5af389445118bed2bdebf7a8a42b219c8fba5175` is discovery-only reference under CC BY 4.0; see [attribution](reference.md). Field names/types, current versions, wire ordering and enum overloads are independently established from this Linux target's DWARF and bytes. No original code was executed and no proprietary project data was written.

## Current grammar

All words are little-endian 32-bit values. Every Religion, BeliefYield and ReligionBuildingYieldModifier begins with version 20. PlotCoord has two signed words without a version. Each vector has one unsigned 32-bit element count followed by ordered elements; duplicates and unknown hashes are preserved.

Religion fields follow this exact writer order:

1. Faith balance Q8 raw signed word; faith rate change Q8 raw signed word; pantheon Belief definition hash; number of beliefs earned; holy-city X and Y signed coordinates.
2. BeliefYield vector; ReligionBuildingYieldModifier vector.
3. Religion-building discount; spread distance; combat loss; inquisition signed word; gains-all-follower-beliefs bool; founder-belief-of-majority-religion bool; faith per peaceful religion founder.
4. Add-faith harvest modifier; add-faith chop modifier; add-faith barbarian-plunder modifier; adopts-ally-founded-religions signed word; treat-ally-capital-as-holy-city signed word; treat-holy-sites-as-holy-city signed word.
5. Spread multiplier; spread-enhancing Tech definition hash; tech-enabled spread multiplier; natural-wonder relic; faith from dispersal; religious pressure per great person activated; enable-wall-attack-whole-game-same-religion signed word.
6. Ordered promotion-class hash/signed-value vector for enable-wall-attack-whole-game-same-religion.
7. Start-inquisition charges; spread trade-route pressure destination modifier; spread trade-route pressure origin modifier; GreatPersonIndividual founder definition hash; units-religion-strength-by-religion signed-word vector.

All fields not explicitly identified as hashes, bools, vectors or coordinates are signed words. The native `m_bInquisition`, `m_bAdoptsAllyFoundedReligions`, `m_bTreatAllyCapitalAsHolyCity` and `m_bTreatHolySitesAsHolyCity` are **int**, despite their names. They are preserved without bool normalization. Yield and BeliefYield enums are raw signed ordinals; Belief, District, Tech and GreatPersonIndividual enums serialize definition hashes rather than SQL ordinals.

| Nested record | Fields after version 20 |
| --- | --- |
| BeliefYield | BeliefYield raw ordinal; Yield raw ordinal; signed amount; signed per-X-items; District definition hash |
| ReligionBuildingYieldModifier | Yield raw ordinal; signed multiplier; one-byte founder-only bool |

Current bools accept only bytes 0 or 1. Legacy reader branches, definition-hash remapping, version-dependent defaults, archive edit/notification behavior, object pointers and callbacks are separate native Read semantics, not wire parsing. No gameplay/RNG equivalence follows from this codec. No fields are classified as planning-only or excluded by name.

## Bounds and validation

`Religion::{decode,decode_prefix,decode_prefix_with_items,encode,encode_with_items}` use `unit::Limits`. Decode is strict about trailing input and all three local version positions. Prefix forms return actual consumed bytes, and item forms aggregate all collection-element charges. Each vector checks item budget, minimum remaining input, checked typed allocation size and fallible reservation before allocating. The byte limit bounds consumed wire bytes and individual vector allocation size; it is not a cumulative typed-heap cap. The existing `unit::HashValue<i32>` representation preserves ordered promotion-class pairs.

Five isolated tests compiled with `-Dwarnings` cover an independent nonempty source carrier, duplicate high-bit hashes, negative Q8 and int-valued b-prefixed fields, both bool values, every byte truncation, all nested/root versions and bool positions, trailing/prefix behavior, oversized count preflight, and exact/one-short byte/item caps.

```sh
python3 tools/linux/player_religion_audit.py --output evidence/linux/player-religion.json
```

The offline audit validates **43 complete original records per immutable Core stream**, across two existing hash-gated streams and owner traces: **17,173 bytes per stream**, with record sizes 394, 418, 442 and 457 bytes. Every record is fully consumed and re-encoded byte-for-byte; exact byte/item limits pass and one-short limits fail. Original records exercise 0–2 BeliefYield entries, 0 or 3 building modifiers, 20 promotion-class pairs and 25 religion-strength values; item charges range from 45 to 49. These traces supply validation cuts only, never production parser input or grammar. Synthetic carriers have no original execution oracle; offline writer bytes do not prove live Read side effects or initialized-game equivalence.

Player's next actual component is **Resources**: call `0x2b4f0ad` through PLT `0x1eb9310`, resolved to writer body `0x2b6b0f0`. Complete gameplay state/RNG, external player decisions, actions, upkeep and turns remain open. The recorded raw Save→Load difference remains **89,198 bytes**, with unknown bytes unclassified and unmasked.
