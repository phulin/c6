# Linux Player AI CityManager saved wire

The bounded [Rust codec](../crates/c6-core/src/player_ai_city_manager.rs) implements the current Linux XP2 CityManager v2 and its complete nested CityBuild v17 saved grammar. This is saved wire transport. Full Player/world import, gameplay state, action execution and turns remain unsupported. Native Read initialization, city/player lookup, callbacks, archive integration and analyzer reconstruction are separate dependencies; this codec does not execute them. No AI state is classified as dispensable or planning-only, and no bytes are masked. The recorded **89,198-byte raw Save→Load difference** remains a failure.

The [reproducer](../tools/linux/player_ai_city_manager_audit.py) and [report](../evidence/linux/player-ai-city-manager.json) gate the supplied Linux `libGameCore_XP2.so` SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. Complete method hashes, direct PLT resolutions, virtual save/load slots and matching target DWARF establish serializer identities. No original game code is executed. Names/layout leads use the attributed [cru121 reference](reference.md), pinned revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; target bytes and DWARF govern the grammar.

## Position and complete current grammar

Player writer `0x2b4ee00` calls CityManager at `0x2b4f267`, after Planning.Manager and before CultureAi. CityManager writer `0x1fcf530` saves version 2 (constant `0x2dfbc20`), signed i32 Player identity and `m_cityAis` at object offset `0x60`. Reader `0x1fcf5a0` reads identity at v2, looks up non-sentinel identities through Player.Manager, may call Initialize, then reads the recycled container. A saved signed identity is retained without resolving a live pointer. The next unframed dependency is **CultureAi v15**.

All scalar words are little endian. Definition hashes retain u32 bits and saved order, including duplicates; enum/hash values are not validated or remapped against a fabricated registry. FixedPointT<8> values retain signed i32 raw storage. PlotCoord is two signed i32 values. CityID and ComponentID both save signed i16 owner, signed i16 component type, and u32 ID, without pointers or padding. Control.Handle is version 1, signed i32 Player identity, u32 ID.

The recycled CityBuild container writer `0x1fd0c90` / reader `0x1fd0b00` saves `u32(0x10001), u32(16), capacity:u32, live_count:u32`, then ordered `(index:i32, CityBuild)` entries, followed by `current_id_bits:u32, free_count:u32, first_free:u32, last_free:u32` and ordered free indices as u32. Capacity is metadata; the codec allocates only actual entries and free-index values. Older recycled formats and block sizes are explicit unsupported forms. Metadata and indices are preserved without simulating AddAt, free-list repair or assigning unsupported semantic constraints.

CityBuild writer `0x1fd2fb0` / reader `0x1fd3360` saves the following sequence, verified against target DWARF offsets and concrete FAutoVariable virtual serializers:

1. Version 17; CityID; city ID u32; owner i32; City AI ID u32.
2. Current Yield, base Yield and PseudoYield hash arrays, each `u32 count; (hash:u32, raw:i32)*`.
3. Best Improvement hash u32; city threat i32; under-attack bool storage; desired and needed defenders i32.
4. Counted version-1 plot reservations `(controller:u32, value:raw i32, tag:u32)`; counted event-tree u32 IDs; working Control.Handle; counted Control.Handles.
5. Done-off-turn bool storage; best plots to clear/buy; counted i32 food-loss amounts, turns and populations; counted UnitEfficiencyInfo v4.
6. Trade.City v3; counted raw i32 yield preferences; counted CityPlotEvaluation v1.
7. Signed i32 evaluation-layer count, then one Building, District and Improvement hash array per layer, in that order. Negative layer counts are explicit unsupported forms.
8. Unit, cached Unit and Project raw-i32 hash arrays; raw i32 power surplus/default build present value; Resource hash/i32 array.

UnitEfficiencyInfo v4 (`0x21bc9e0` / `0x21bcb20`) saves Unit hash, max damage i32, seven bool-storage bytes (obsolete, ranged, naval, zone-of-control, unique, excluded, can-build), five i32 values (speed/range/strength/defense/number in use), two raw fixed-point aggregate efficiencies, counted per-Player raw i32 efficiencies, raw efficiency, Resource hash and resource amount i32. Its native inline capacity 64 does not introduce a fixed wire array or justify assuming a count of 64.

Trade.City v3 (`0x21b4620` / `0x21b46a0`) saves CityID, current route/unit counts i32, counted Trade.CityTradeConnection v1 records and counted failed CityIDs. Connection v1 (`0x21b4580` / `0x21b45d0`) saves CityID, RailConnection bool storage and raw i32 RailValue. This is the AI trade record; it is not the actual City.Trade subsystem.

CityPlotEvaluation v1 (`0x1fd2d20` / `0x1fd2e60`) saves index i32, PlotCoord, evaluate-wonders bool storage, ComponentID, Resource hash, Feature hash, location PlotCoord, counted six signed i16 neighbor values, then twelve bool-storage bytes: improvement, district, pillaged improvement, removable feature, build improvement, build district, place building, purchase, neighbor district, revealed, owned and worked. Pointer-valued evaluation arrays and reservation pointers in live CityPlotInfo are omitted by this serializer, not classified as unnecessary gameplay data.

## Raw bool storage evidence

The original `OStream::WriteObject(bool const&)` at `0x1fe7900` forwards the pointed-to **one byte** to the stream virtual write, without canonicalizing it. Each original canonical stream contains **1,127 noncanonical bool bytes** in PlotEvaluation: 1,067 HasDistrict bytes and 60 EvaluateWonders bytes. Value **197** occurs 12 times per stream. The first is Player 0, CityManager-relative offset 175,802 (absolute Core offset 5,409,959; CityManager starts at 5,234,157). The codec therefore exposes `BoolByte(u8)` and preserves every bit; it does not convert these bytes to Rust bool or infer their logical truth. Gameplay interpretation still requires consumer/getter evidence.

For HasDistrict specifically, writer `0x1fd2d20` uses object offset `0x51` at call site `0x1fd2dc9`; matching reader `0x1fd2e60` reads object offset `0x51` at `0x1fd2ef9`. Both bind the one-byte bool overload. The independent Python walker reconstructs all preceding typed fields, validates the neighbor count of six and all nested versions, matches every nested UnitEfficiencyInfo span against original owner-trace extents, and reaches the next CultureAi version 15. The report lists each noncanonical byte with its exact record-relative position and field path, plus its value distribution. These checks establish the saved-field boundary independently of the Rust codec; they do not establish runtime truth semantics or whether native memory was initialized.

## Validation and limits

The two SHA-gated existing canonical streams each contain 43 enclosing Player v47 records, 245 CityBuilds, 9,065 PlotEvaluations and 11,547 UnitEfficiencyInfo records. Their CityManager payloads total 18,881,812 bytes per stream. Their owner traces have **zero CityManager/CityBuild hooks**. This is trace instrumentation coverage, not absence of serialized CityManager state: validation starts at each genuine Planning.Manager end and walks the entire CityManager grammar. Every record matches the enclosing Player identity, every nested UnitEfficiencyInfo owner extent matches, and the next word is CultureAi version 15. Production decoding contains no fixture offsets, hook logic or byte searching.

The source fixture exhausts all byte prefixes of a small nonempty carrier, with nested records, signed extremes, raw bool value 197, shuffled free indices and duplicate/high-bit hashes. Actual canonical testing covers every full record, every nested sequence count/data/end and CityBuild boundary, representative in-field cuts, exact and one-short cumulative byte/item budgets, strict trailing refusal and exact prefix accounting. It **does not exhaust every byte prefix of megabyte-sized actual records**; the report records the actual cut counts. Allocation checks use checked arithmetic, addressable owned sizes, fallible reserves and all affected wire/item budgets before reservation. Encoding measures the complete record before reserving output. Unsupported current forms fail explicitly without mutating caller state.

The committed report validates **86 original records and 256,732 structural/representative truncation cuts** against source commit `50b849ac115ee82840b0ff55d25875e4a2755446`, with three actual Cargo module tests and strict c6-core Clippy passing. All 1,228 archived source files remained unchanged. The candidate driver builds an unchanged `git archive` of its explicit source commit and imports the real exported `c6_core::player_ai_city_manager`; a missing export fails compilation. It does not inject modules into an archived library. It verifies every archived source hash before/after build, runs the actual Cargo module tests and strict Clippy, and records archive/module/driver/harness/executable identities. The two existing original Save outputs are structural oracles; no new native Read oracle or gameplay parity is claimed.

Native CityBuild Read also resolves the City, calls Initialize, initializes plot analyzers based on location/version, marks its evaluation job updated, attaches CityBuildAI to the City, initializes lists and its archive. CityManager Initialize subscribes signals. These live effects, callbacks and potential transitive gameplay/RNG consumers remain open. No direct-name call screening proves transitive RNG purity, and external policies must supply all player decisions.

Reproduce without original-code execution:

```sh
python3 tools/linux/player_ai_city_manager_audit.py --source-commit 50b849ac115ee82840b0ff55d25875e4a2755446 --output target/linux/player-ai-city-manager.json
```
