# Current Linux City wire import

The production [City codec](../crates/c6-core/src/city.rs) composes the complete current City v51 serialized record from typed component codecs, including inherited properties and its final fields. It preserves all fields in the two original native fixtures and refuses unsupported versions, nested variant forms and resource excesses. This is a wire import milestone: complete live City state, native Read fixups, a full Player/Core importer and City gameplay execution remain unsupported.

The [reproducible audit](../tools/linux/city_wire_audit.py) and [checked report](../evidence/linux/city-wire.json) gate Core SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and matching Linux host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. They check 121 original method fingerprints, current versions, inherited host virtual bindings, City caller bytes and matching DWARF. Research reused the pinned cru121 reference revision `5af389445118bed2bdebf7a8a42b219c8fba5175` under CC BY 4.0; [reference attribution](reference.md) applies. Original Linux evidence establishes the contracts; Windows layouts, SQL indices and public build numbers are not behavior evidence.

## Composed record and APIs

Original writer `0x2890f30` and reader `0x28913c0` serialize the current record in this order:

| Record | Current version | Production codec |
|---|---:|---|
| Complete pre-component header | 51 | [city_wire.rs](../crates/c6-core/src/city_wire.rs) |
| Districts | 4 | city_wire::Districts |
| Citizens | 2 | [city_citizens.rs](../crates/c6-core/src/city_citizens.rs) |
| Culture, Gold, Growth | 17, 11, 23 | [city_economy.rs](../crates/c6-core/src/city_economy.rs) |
| Religion, Resources | 5, 10 | [city_services.rs](../crates/c6-core/src/city_services.rs) |
| BuildQueue, Buildings | 16, 10 | [city_production.rs](../crates/c6-core/src/city_production.rs) |
| Combat, Trade, CulturalIdentity, Power | 7, 12, 14, 10 | city_services |
| Inherited TypedVariantMap properties | Existing keyed framing | unit::variants |
| Final fields | No separate record version | city::City |

The final fields are AI controller ID u32, founded-turn i32, counted Improvement definition hashes, counted population-change i32 values and counted Yield hash/i32 Power-modifier pairs. The header's City AI ID is a separate preserved field. Their names do not establish a planning-only exclusion. The final virtual call is the inherited `m_kProperties` value at City `+0x1a8`, writer slot `+0x1f8` with flag 1; matching host table `0x53b3680+16` binds it to `0x3961e50`, with reader `0x3971330`. It is not a desktop UI or native policy-planning call. Header `m_kAttributes` is a distinct **untyped** VariantMap; it must not be conflated with these typed properties.

`City::decode_prefix_with_items(bytes, unit::Limits)` returns typed data, consumed byte count and aggregate dynamic-item charge. `decode_prefix` omits the charge; strict `decode` additionally requires exhaustion. The encoder likewise offers `encode_with_items` and `encode`. Every child codec has matching prefix/item APIs and retains its strict decoder behavior. The parent supplies the remaining byte/item credit to each component, so a long record cannot spend the same item budget again at each boundary. One component's encoded buffer is appended and dropped; no world or complete City object is cloned to count fields. The model owns the decoded record's logical wire vectors and strings; native addresses are not imported.

The existing recursive `map::LocalizedText` parser is reused for Building names through budgeted prefix/encode wrappers. It preserves Linux long64 values, float/double bit patterns, recursion and named-argument metadata. Prefix clipping respects the caller's byte limit; aggregate item charges are returned to Buildings and the full City. Existing Map strict decoding and parser internals are unchanged. The original complete 4,693,980-byte Map roundtrip was rerun successfully after exposing the wrapper.

## Important identity and field distinctions

The [movement registry contract](linux-city-district-state.md) remains separate. City.Districts contains actual full **District instance IDs**, not definition types. Runtime lookup uses valid owner plus low-16 slot, retaining requested and resolved IDs; global District records/definition mappings are still needed to answer movement queries. Serialized City state does not supply the omitted live domain mask by itself.

The header includes all current scalars, a 64-element int array with **no count prefix**, fourteen ordered Yield hash/value arrays, six typed modifier-record vectors, byte-string name and VariantMap. FixedPointT<8> fields preserve signed raw Q8 words. Each enum provider was resolved individually: Terrain, Improvement and GreatPersonClass examples use hashes, while generic Yield and GreatWorkObject examples use raw signed indices. An important original asymmetry remains explicit: HappinessModifiers writes its Happiness ordinal through scalar int, but reads it through HappinessTypes HashToIndex. A lossless wire decoder must retain the written value; it must not silently claim native load normalization.

BuildQueue preserves every outer/inner type hash, raw OrderTypes and flags, request ID and location. Native order tags 0–3 have their actual typed payloads; the original writer and reader give other raw order values **no branch payload**, which is represented explicitly. Nested queue nodes use version 2. Buildings includes indexed modifiers, GreatWork slots, Feature yields and recursive localized Building names. Building/Wonder yield modifiers save Yield hashes, whereas the nested Feature-yield record writes a raw Yield index; these are different fields.

Religion and Power include their complete current nested records. Resources saves its City ComponentID with signed i16 owner, i16 tag and u32 ID. Trade retains its sequential bool posts and fourteen indexed Q8 arrays. CulturalIdentity retains player, conversion and loyalty data without a blanket AI exclusion. Nothing in this codec executes a production choice, native AI decision or turn update.

## Original-byte validation and structural progress

The audit builds a **separate offline Rust candidate**, records its source/binary identity, and reads the existing immutable native streams and owner traces. No original game code executes. Traces select validation records and cross-check boundaries only; the structural indexer does not consult them.

Both original streams contain 245 complete City records. All **490** records roundtrip byte-identically, including all twelve individually traced child components per City: **5,880** independent native child boundaries. The first stream's City records total 4,643,560 bytes. Every complete record additionally checks prefix/trailing behavior and exact/refused cumulative byte/item budgets using its real data. Header/Districts and Citizens fields are compared to an independent typed Python decoder; header attribute bytes are compared without masking, while a separate attribute-AST comparison is not claimed. Nested variant forms remain subject to the explicit gates in the existing variant codec.

The optional `--first-city` mode in [the Core indexer](../tools/linux/core_stream_index.py) follows byte 0 through complete Map, Religion, first Player/Formations and the accepted Goody_Hut/Bonuses/Congress grammars, then Player.Cities v15/recycled-container v65537 framing. It reads the first actual slot and typed City header, Districts and Citizens, stopping at Culture. On the existing fixtures the first City starts at 4,699,232 and this partial structural stop is 4,700,814. These are **computed validation results**, never parser constants or signature searches. Default Map-only, Formation-only and Player-component-only modes retain their earlier stops. If the first player's City container is empty this mode stops explicitly at its unparsed container tail rather than fabricating a City.

The standalone Rust City codec already consumes the complete record from a structurally provided cursor. A direct Rust byte-0 Core reader is the next integration step; callers must not depend permanently on Python-generated offsets or traces. Full Player.Cities closure also requires its exact surrounding container metadata and later fields, followed by the next real unframed Player component. The present partial structural stop does not imply complete City/Unit roster indexing.

Reproduce the City profile with:

```sh
python3 tools/linux/city_wire_audit.py --output target/linux/city-wire.json
cargo test -p c6-core city
```

## State and gameplay limits

Current field coverage is serialized wire state. It does not reproduce parent-pointer rebinding, Resources.Initialize(saved City lookup), reconstructed domain masks, archive dirty history, reporting registrations, constructor defaults, lazy gameplay cache effects, or other original Read side effects. Existing runtime state cannot be synthesized from zero or excluded merely because it has an AI name. Proven presentation-only caches/addresses need not reproduce native heap layouts, but gameplay consumers, ordering, all gameplay RNG and external policy decisions still require exact support.

Keeping unknown downstream bytes untouched does not authorize advancing them. The 89,198-byte raw Save/Load difference remains preserved and unmasked. Complete live world import, general actions, EndTurn and whole-state/RNG parity remain unimplemented; no UI bootstrap or full desktop admission is introduced as a simulator prerequisite.

Checkpoint validation passes workspace tests (151 core tests), strict workspace Clippy and the C ABI smoke check. The original Map/Formation index audit still passes both fixtures with unchanged boundaries. The exact tested offline City candidate, report and public source copies are preserved locally under `target/frozen/city-wire/26b8ad4192bd1dff6118827da308f614c88b6dff615a4d3d4c20f5b5652210a8/`; this is distinct from all frozen movement profiles and is not a private upload.
