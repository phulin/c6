# Linux Player Goody_Hut, Bonuses and Congress components

The production [Player component module](../crates/c6-core/src/player_components.rs) decodes and re-encodes complete **current wire records** for Goody_Hut v1, Bonuses v4 and Congress v12, including Congress's nested ResolutionEffectRefund records. This advances structural world import through the first Player's complete Congress record to the beginning of Cities. It does not decode a complete Player or City, locate the complete Unit roster, execute gameplay, or complete world import.

The [reproducible audit](../tools/linux/player_components_audit.py) and [report](../evidence/linux/player-components.json) require original Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. The audit checks 37 complete original methods, current version constants, concrete Reset virtual bindings, selected read/reset bytes and matching DWARF. It reuses the reviewed Map parser and the existing original stream/owner evidence, then builds and runs an **offline Rust codec example**; no original game code executes. The pinned cru121 reference revision `5af389445118bed2bdebf7a8a42b219c8fba5175` supplied discovery leads under CC BY 4.0; [reference attribution](reference.md) applies. The supplied target's bytes and matching DWARF establish these contracts; SQL rowids and cross-platform layouts are not substituted.

## Exact current wire grammar

All scalars are little endian. An indexed-int array is `u32 count`, followed by exactly that many ordered `(u32 definition_hash, i32 value)` pairs. Counts are wire counts, not destination registry lengths. Unknown and duplicate hashes remain represented in the wire codec. Encoding this wire representation preserves the original order and values, rather than manufacturing a normalized native Save.

| Component | Current wire order |
|---|---|
| Goody_Hut v1 | i32 version; indexed-int PlayerGoodies using GoodyHut definitions |
| Bonuses v4 | i32 version; indexed-int FlatBonuses using GovernmentBonusName definitions |
| Congress v12 | i32 version; six favor ints; three indexed-int MinorCivBonus arrays; counted raw int GovernmentSlot vector; fourth indexed-int MinorCivBonus array; four modifier ints; counted Refund records; bool WantsCongressToMeet; i32 DiploVPLastTurn |
| Refund | i32 version **12**; u32 Resolution definition hash; i32 WhichEffect; i32 RefundPercent |

The Congress favor fields are Player, Lifetime, EnteringCongress, ExtraPerTurn, SuzerainMultiplier and AllianceMultiplier. Its first three indexed arrays are SuzerainFavorByType, InfluenceTokenFavorByType and TradeRouteFavorByType; the fourth is TradeRouteByTypeModifier. The four later modifier ints are EmergencyTarget, EmergencyMember, GreatPerson and BidCost. The GovernmentSlot vector is serialized by position, **without definition hashes**; do not turn it into an indexed hash overlay. Refund's version is the same current version constant as Congress, not version 1. ResolutionTypes uses original WriteIndexToHash/HashToIndex, not a saved live ordinal.

The current Goody_Hut writer/reader are `0x2b06130`/`0x2b06180`; Bonuses are `0x2a562e0`/`0x2a56330`; Congress are `0x2a65480`/`0x2a65600`; Refund are `0x2a653e0`/`0x2a65430`. Concrete indexed-int GoodyHut providers `0x2b07790`/`0x2b07820` establish the count/pair framing; the audit also pins the GovernmentBonusName and MinorCivBonus providers and the actual definition lookup/index providers. Matching DWARF identifies the refund's 12-byte live layout and all named component fields. No native padding or allocator capacity is copied into the wire record.

Only these current versions are supported. Historical Bonuses/Congress readers have additional version-dependent branches; this codec refuses them rather than applying the current grammar to old records. Invalid bool encodings, truncations, trailing bytes and resource overflows are explicit errors. `decode_prefix` reports consumed bytes for an already structurally located component; strict `decode` requires exhaustion. It does not search for version signatures.

## Reset defaults and read application are separate

The `DefinitionLookup` input must implement the genuine active native collection's Find/GetCollectionIndex contract, with pinned rules/DLC/mod configuration. It includes direct-index handling for small keys and the original duplicate-hash selection. Raw wire pair position alone is not a general import ordinal: a valid read can reorder pairs, omit hashes, or contain duplicates. A canonical original writer iterates the actual definitions in collection order, which can supply additional registry evidence under that exact configuration; SQL indices are not used here.

`apply_indexed_ints` starts with explicit existing destination values. It maps each pair using that actual registry, consumes unknown hashes without changing a destination, and lets later mapped pairs overwrite earlier ones. The destination size must be supplied consistently; an unavailable preexisting value is not a zero default. Errors return no partially usable result and do not mutate the supplied prior state.

- **Goody_Hut Read does not call Reset.** It overlays existing PlayerGoodies and leaves the omitted `m_applyingGoody` flag unchanged. `GoodyHutState::after_reset` is separately supported because original Reset explicitly clears that flag, clears the vector and resizes to the actual GoodyHut definition count using explicit int zero. `GoodyHut::apply` preserves the supplied flag.
- **Bonuses Read calls its concrete Reset first.** Original Reset clears FlatBonuses and resizes to the actual GovernmentBonusName definition count with explicit int zero; the v4 reader then applies its hash overlay. `Bonuses::apply` performs those verified value rules, independently of previous vector values.
- **Congress Read resizes each indexed array to the current MinorCivBonus count, then overlays it.** It does not call Congress.Reset. The concrete `Types::Vector<int>::resize(unsigned long)` at `0x20599d0` preserves the existing prefix and contains no int initialization stores when exposing new slots. `apply_resized_indexed_ints` therefore keeps known prior slots and treats newly unavailable slots as unknown until a mapped wire value supplies them. If a slot still lacks a value it returns `MissingResizeValue`; allocator contents are not synthesized. An explicit prior initialized slot can satisfy this dependency. Truncated excess prior values follow the native resize result.

Congress.Reset is a separate operation: it clears the refund vector and rebuilds its indexed and GovernmentSlot vectors with explicit zeros, but it does **not** reset all scalar fields. The wire codec and indexed-array application do not silently act as a full Congress constructor/Reset. Omitted `m_iLastTurnConsideredCongress` and `m_iCongressStageCompleted` must remain in the owning Player's actual runtime state. Their absence from Save does not prove a planning-only exclusion. FAuto archive/dirty effects, component ownership and other omitted live state remain outside this value codec; it is not full Player.Read lifecycle equivalence. The current Refund helper resolves its Resolution hash through the actual registry and returns -1 on a native miss while retaining the raw hash in wire state.

Allocation is bounded by the caller's existing codec `Limits`, checked element counts and fallible vector reservation. Indexed application copies only the affected component vectors, not the world. No complete City, District, Player or gameplay state is cloned or advanced by these helpers.

## Trace-independent boundary progress and original-byte validation

Use `index_core(data, first_player_components=True)` or:

```sh
python3 tools/linux/core_stream_index.py CORE.bin.gz --first-player-components --output INDEX.json
```

The default Map-only mode and old `--first-formations` mode retain their existing stops. The new mode follows the same complete Map and Religion grammar, first Player prefix and Formation container, then consumes the three schemas above. It returns distinct ranges for `first_player_goody_hut`, `first_player_bonuses` and `first_player_congress`. It uses **no traces, fixed fixture offsets or byte-pattern searching**. The [Core stream contract](linux-core-stream-layout.md) remains the surrounding grammar.

Both existing 80,657,462-byte original normalization streams yield these observed ranges after the parser independently computes them:

| First-player record | Computed range | Bytes | Observed contents |
|---|---|---:|---|
| Goody_Hut | 4,698,763..4,698,827 | 64 | seven indexed pairs |
| Bonuses | 4,698,827..4,698,923 | 96 | eleven indexed pairs |
| Congress | 4,698,923..4,699,208 | 285 | four arrays of six pairs; five GovernmentSlot ints; zero Refunds |

The newly reached **Cities start is 4,699,208** on these fixtures. These numbers are validation results, never general parser constants. The Congress start/end and following Cities start independently match existing original ownership traces. There is no separate Goody_Hut/Bonuses owner observation in those traces; their source-derived framing is cross-checked by reaching the exact following Congress boundary. Adding a counted pair synthetically moves the following Congress/City boundary by eight bytes without changing the Congress payload digest.

The audit passes the dynamically extracted first components to the production Rust codecs, and independently checks every existing Congress owner record: 43 per stream, **86 native records**, all byte-identical after decode/encode. Every observed native Refund vector is empty. Nonempty Refund framing is pinned to original serializers and tested with source-only cases; no dynamic nonempty Refund parity is claimed. Rust tests also cover signed/extreme values, unknown and duplicate hashes, prior-state preservation versus Reset, unknown resized slots, all truncated Congress prefixes, wrong versions, bad bools and resource refusal.

Reproduce with:

```sh
python3 tools/linux/player_components_audit.py --output target/linux/player-components.json
cargo test -p c6-core player_components
```

The offline example and its complete tested source identities are recorded independently from the frozen movement native candidate. No native rerun, saved-Begin/UI path, private upload or Ghidra change is involved. Full Unit-roster indexing still requires the unframed Player schemas after this Cities boundary; complete City records are the immediate next codec. Complete logical gameplay state/RNG and all external player decisions remain required. The existing 89,198-byte raw Save/Load difference remains unmasked, and wire roundtrips do not establish state/action/turn parity.

Checkpoint checks pass: `cargo test --workspace` (105 core tests, including four focused Player component tests), `cargo clippy --workspace --all-targets -- -D warnings`, and `bash tools/check/c_abi_smoke.sh`. The preexisting 42-method Core index audit also passes on both original fixtures; its only report change is the indexer source digest, with old Map/Formation boundaries unchanged.
