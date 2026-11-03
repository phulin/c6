# Linux Map and Plot state contract

The independent [`map_state_audit.py`](../tools/linux/map_state_audit.py) decodes the complete observed original Map v4 serialization, including its six named component payloads, directly from original Linux Core streams. It resolves all 6,996 Plot links to Area, Landmass and Region object IDs and checks 315 independently calculated boundaries against the original Save owner trace. This establishes a substantial typed world-state transport contract. Complete gameplay-state reconstruction, movement, actions and turns remain unimplemented; codec agreement does not establish their behavior.

The original streams before and after Load contain **identical Map bytes**. They are two lifecycle observations of the same map, not independent map-generation or gameplay branch fixtures. The complete streams still differ by **89,198 bytes**. Unknown bytes elsewhere remain unclassified and unmasked.

The audit requires supplied Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; original streams have SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884` and `85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8`. Their existing fixture/configuration limits remain those in [Linux game loading](linux-game-load.md). No native game execution or initialized world oracle is added here.

## Structural discovery and evidence

Discovery starts with cru121's pinned revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, `reference/data/layouts.json`, specifically `Map::Instance`, `Map::IInstance` and `Map::Plot::Instance` plus their reflection structures. Matching supplied-module DWARF queried through read-only GDB corroborates these layouts and supplies missing nested fields. Fresh supplied-module instructions establish field order, types, omitted fields, version branches and ID accessors. The upstream material is a discovery lead, not behavioral acceptance; see [reference reuse](reference-reuse.md). Attribution: cru121, CC BY 4.0 for reference findings; no upstream implementation or proprietary raw dump is copied here.

Private raw evidence is under `../c6-ghidra/evidence/linux-map-state/`; [map-state.json](../evidence/linux/map-state.json) records its checksums. The original owner traces remain in `../c6-ghidra/evidence/linux-save-normalization/`. The audit selects the outer `GameCore::Map::Instance` owner from that trace, then computes every nested boundary from native field order and serialized lengths. It contains no fixture section offsets. Original trace ranges validate the Visibility instances/manager, Areas, Landmasses, Regions, Continents and UnitContainer. Named component payloads use private `MemoryOStream`s, whose inner owner ranges are absent from this outer-stream trace; component agreement is checked against the exact length-delimited envelope, not invented child trace observations.

| Native boundary | Linux VA |
| --- | --- |
| Map Write / Read | `0x2a46df0` / `0x2a47010` |
| Plot-vector Write / Read | `0x2a47240` / `0x2a472c0` |
| Plot Write / Read | `0x2bc84d0` / `0x2bc8820` |
| Plot serialization version, returns 6 | `0x2bc8c60` |
| Plot CityAttributes Write / Read | `0x2bc8030` / `0x2bc8090` |
| Map component writer | `0x2a45ef0` |
| Area / Landmass / Region / Continent writers | `0x2d00dc0` / `0x2d025c0` / `0x2d027d0` / `0x2d01750` |
| Visibility manager / instance / layer writers | `0x2bc0f50` / `0x2bba130` / `0x2bba970` |
| Feature / River / Terrain / Territories writers | `0x2dd35a0` / `0x2de66e0` / `0x2df02b0` / `0x2d1ea10` |
| UnitContainer / sparse component-ID grid writers | `0x2a4dab0` / `0x2a4db70` |

All wire scalars are little endian. `i8` records use the native signed `char` value; masks preserve the same eight bits as `u8`. `bool` is one byte. A counted vector has a `u32` element count; this also applies to `WriteArray`, including arrays whose native length is fixed. A definition hash/value array is `u32 count`, then ordered `(u32 type_hash, i32 value)` pairs. Preserve order, IDs, sentinel values, aggregates and complete metadata.

## Map v4 sequence

1. `i32 version=4`; four `i32`: width, height, top latitude, bottom latitude; two `bool`: wrap X, wrap Y; `u32 MapSizeTypes` hash.
2. Three `i32` aggregates: land plots, owned plots, natural wonder count.
3. Resource-count hash/value array and land-resource-count hash/value array.
4. Player visibility manager, Area container, Landmass container, Region container, counted Continent vector, FeaturePlots.
5. Plot vector: `i32 Plot version=6`, `u32 count`, consecutive Plot records without per-Plot versions.
6. UnitContainer, then the named map-component envelope.

The observed map is 106 × 66 with latitude limits 90 / −90, neither axis wrapping, 5,749 land plots, saved owned-plot aggregate zero and seven natural wonders. There are 43 visibility instances, 126 Areas, seven Landmasses, 131 Regions, 254 Chokepoints, six Continents and four FeatureLayouts.

Historical reader branches are visible in the target: Map v1 consumes two obsolete integers after initialization; FeaturePlots is present for Map version ≥3; components are read for version ≥4. The current audit accepts version 4 only. Map Read calls Initialize from the dimensions/wrap/latitude parameters before reading containers and Plots; it does not deserialize native pointers or raw object layouts.

## Plot v6 record

| Order | Wire type | Meaning |
| --- | --- | --- |
| 1–3 | `i32`, `i32`, `u32` | Area ID, Landmass ID, Region ID (`0xffffffff` for absent Region) |
| 4–7 | `u32`, `u32`, `u16`, `u32` | Terrain hash, Feature hash, FeatureLayout index, Continent hash |
| 8–10 | `i8`, `u32`, `u16` | Unit count, Resource hash, resource quantity |
| 11–15 | `u32`, three `i8`, `i16` | Improvement hash, improvement owner, route type, route era, appeal |
| 16–21 | six bytes | River E/SE/SW flow directions; river-crossing count; river-crossing mask; cliff-crossing mask |
| 22 | three bytes | Little-endian serialized flags below |
| 23 | `u32` | Property count; nonzero is followed by a normal `TypedVariantMap` |
| 24 | optional 17 bytes | CityAttributes when serialized flag 14 is set |

Minimum record size is 55 bytes. Flags 0–16 are improvement pillaged, route pillaged, route bridgey, starting plot, hills, NE/W/NW of river, NE/W/NW of cliff, rough feature, adjacent to land, impassable, has CityAttributes, feature added, feature volatile. Native Write clears the 24-bit buffer and sets these 17 bits; native Read consumes 24 bits for version ≥5 but reads only 17 meaningful bits. The stable bounded contract rejects nonzero spare bits rather than silently normalizing them. Plot versions ≤4 have different flag widths/versioned fields and are unsupported by the audit.

CityAttributes contains three `u32` IDs (working city, owning city, district), `i8 player`, and `u32 Wonder Building hash`. Its native `validDomains` and plot-yield-effect fields are omitted. Native Plot Read re-creates a local CityAttributes allocation index; the allocator index itself is not serialized. Six observed Plots have nonempty property maps, including a nested typed-map branch; 5,287 have CityAttributes. Property wire uses the existing [typed variant contract](linux-unit-wire-variants.md). The audit independently traverses supported nested metadata and fails on unsupported branches. A nonzero outer property count merely controls the native branch; strict supported transport requires agreement with the map's own entry count.

Native Plot objects are 80 bytes: vptr, Area/Landmass/Region/property pointers, 20-byte Constants, 18-byte Variants, local CityAttributes index. Pointer values are never save identities. Coordinates are omitted from Plot Write. `InitPlots` (`0x2a47f40`) initializes X in the inner loop and Y in the outer loop; `GetPlotIndex(int,int)` (`0x21436b0`) computes `y*width+x`; `GetPlotXY` (`0x21a3ee0`) uses signed quotient/remainder. `GetPlot(int)` (`0x2a49730`) indexes the dense vector after checking its size. Thus for supported nonnegative in-bounds indices, `x=index%width`, `y=index/width` is the actual identity relationship.

`GetPlot(x,y)` (`0x2a48a30`) applies native `CoordRange` independently to each axis, checks `IsPlot`, then indexes the vector. `CoordRange` (`0x243bb00`) returns the input when wrapping is false; for a negative wrapped input it returns signed C++ remainder plus axis size. A negative exact multiple therefore produces the axis size and fails the later bounds check. It is not ordinary Euclidean modulo. This is static evidence; no wrapped-map native fixture is added.

Political ownership is distinct from the geographical Territories component. `Plot::GetOwner` (`0x1fe23d0`) calls `CityAttributes::GetOwningCity`; that method (`0x1fe2460`) constructs a `CityID` from signed player byte and owning-city `u32` ID. `ComponentID::GetOwner` (`0x1fe6b70`) returns its signed 16-bit owner field. Do not infer an owner by bit-slicing the city ID. The 5,287 present CityAttributes have player values 0–62, despite the saved map owned-plot aggregate being zero. Preserve that observed aggregate; this lower-Core fixture does not prove a synchronized full-world recount invariant.

## Nested containers and omitted state

Area/Landmass/Region/FeatureLayout collections use `RecycledIDContainer`: `u32 version=65537`, `u32 ID-index bits=16`, capacity, live count; then each live entry's signed slot index and record; then four `u32` values (current generated-ID bits, free count, free head, free tail), followed by the counted number of free-link values. The free values have no additional count prefix. Native `m_uiLastIndex` is not written directly. Preserve allocator metadata and order; a live slot index is not the generated object ID. Region slot 0, for example, stores object ID 65,536. Plot links use object IDs. The audit validates membership against those IDs; it does not synthesize generations or allocator state.

Area container version is 1; each Area record version is 2. Its eleven `i32` values are ID, plot count, coastal plot count, coastal hill/mountain count, owned plots, river edges, units, cities, total population, starting plots, natural wonders. Next come five counted arrays of 64 `i32` values: units/cities/population/free specialists/revealed plots by player; 64 Yield hash/value arrays; Resource and Improvement hash/value arrays; four `i32` boundaries in **east, north, south, west** wire order; water/mountains/canyons booleans.

Landmass container and record versions are 1. Record: ID, plot count (`i32`), water (`bool`), centroid delta (two `i32`). The native final centroid is omitted. Region container version 1 holds two recycled collections. Region record v1: `u32 id`, `i32 count`, `bool chokepoint`, `u32 Area ID`, counted `u32` joined-Region IDs. Native continent type is omitted. Chokepoint record v1: `u32 id`, `bool initialized`, counted endpoint coordinates, counted chokepoint coordinates, counted joined-Area IDs. Coordinates are two `i32` values each. Continent record v1: `i32 id`, type hash, `i32 plot count`, `u32 fertility`; its boundaries are omitted.

FeaturePlots v1 contains a recycled FeatureLayout collection. A layout is `u16 lowest plot`, counted `u16` plot indices. Visibility manager v1 is a counted sequence of `i32 player ID` plus Visibility instance v6. Instance wire: version, player ID, cached revealed-hex count, stale-cache flag, counted revealed bytes, explicit-layer vector, outgoing visibility vector, outgoing visibility-blocker vector. A layer has four `u32` values (ID, index, count, grid width) plus a counted `i16` count-data array. Outgoing records each have v6, `i32 player`, `u32 count`. Native dimensions/wrap/parent pointers, combined layers and initialization state are omitted from this record and depend on initialization/rebuild paths.

UnitContainer v1 contains width, height, a counted vector of layer entries: each has layer ID, sparse-grid v1, allocated-cell count, maximum index, then for each allocated cell its plot index and counted eight-byte ComponentID entries: signed `i16 owner`, signed `i16 component type`, then `u32 ID`. The codec retains the first two halves together in `UnitReference.owner` for wire compatibility; `player()` and `component_id()` expose their meanings. The former plain-player description was incorrect; see [the corrected placement/lookup contract](linux-unit-placement.md). Native order is retained: the fixture's layer IDs are `0,1,3,2`, not sorted. Pointer grids, allocator blocks and the empty local vector are not wire payloads.

## Six map components

The envelope is `u32 version=1`, `u32 component count`, then each component's `u32 name length`, name bytes without a terminator, `u32 payload size`, payload bytes. This framing establishes boundaries and names, not permission to advance unknown payloads. The audit rejects unknown names/versions and requires each supported component decoder to exhaust exactly its framed payload.

| Component | Current contract and observed branch |
| --- | --- |
| Route | v1, portal-manager v1, width/height/count. Actual portal count is zero. For a nonzero count, a shared record version 2 occurs once before all records; each record is entrance plot index (`i32`), counted exit-plot indices, counted source-plot indices. The inherited live Area list is omitted. Nonempty source schema is decoded but lacks native fixture coverage. |
| Feature | v7, sparse extra-Yield data, counted Volcanoes, counted natural-wonder Volcanoes, sparse plot-to-Volcano data, counted Ice records. Actual counts: 23 Volcanoes, one natural wonder, zero Ice. |
| River | v4, next River ID, counted River records, sparse plot-to-River data, sparse floodplain-to-River data. Actual count: 22 Rivers. |
| Terrain | v6, six counted vectors: coastal lowlands, deserts, mountain ranges, named lakes, named oceans, named seas. All six are empty here. Their general current-version record schemas are source-derived and decoded; nonempty native fixture coverage is absent. |
| Territories | v1, recycled geographical Territory records, counted plot-to-Territory IDs. Actual live Territory count zero; all 6,996 plot IDs are `0xffffffff`. General Territory v2 records are decoded from the source-derived contract below; nonempty native fixture coverage is absent. |
| TerritoryBuilder | Zero-byte payload. Builder vtable relocation at `0x3af3c80` selects `Map::Component::IInstance::Write` (`0x2a426f0`), which returns true without writing bytes. This proves the writer's empty serialization, not that the object's live state or upkeep can be excluded. |

`SparsePlotData<T>` v4 serializes default T, counted `i32` plot-to-value indices, counted T values. Const lookup (`0x2de9c60`) returns default for an empty index array, negative/out-of-range plot index, or index value −1; otherwise it selects the corresponding value. It does not establish a generic one-value-per-plot representation. Extra Yield T is the ordered Yield hash/value array; other sparse map component T values are counted `i32` vectors.

A Volcano v5 serializes NamedVolcano hash, plot index, discovering-civilization hash, active turn, counted UTF-8/string8 name bytes, counted Eruptions, counted affected plot indices, counted fixed-point raw eruption periods and chances. NaturalWonderVolcano v5 serializes Feature hash, counted plot indices, counted Eruptions. Eruption v7 is turn, duration, severity (`i32` each). Ice v2 is plot index and RandomEvent hash. River v4 serializes NamedRiver hash, River ID, counted plot/floodplain indices, discovering-civilization hash, counted string8 name bytes, counted Floods, counted edge pairs, counted fixed-point raw periods, counted fixed-point raw chances. Flood v5 is turn, duration, severity and mitigated byte. Edge pairs are two `i32` values using the PlotCoord transport; they are retained as native pairs, not interpreted as ordinary coordinates. FixedPointT<8> wire is its signed `i32 GetRaw()` value (`0x2c9afe0`).

The source-derived general Territory v2 order is ID, TerrainClass hash, plot/coastal/coastal-hill-mountain/river-edge/natural-wonder/feature counts, lake/sea booleans, east/north/south/west boundaries, centroid delta, Feature hash/count array, Localization::String name. Centroid is omitted. The current `Localization::String` writer (`0x2d4e3c0`; reader `0x2d4e6e0`) emits `u32 0xffffffff`, current format byte 1, `u32 base length`, base bytes, `u8 argument count`, argument records, then `u8 named-argument count`. An argument begins with a byte tag: 0 has no payload, 1 has `i32`, 2 has native Linux `long` (`i64`), 3 has `float` raw 32 bits, 4 has `double` raw 64 bits, 5 recursively contains a localized string. The jump table at `0x2f88e6c` establishes the dispatch; `WriteObject(long)` (`0x2add8f0`) explicitly writes eight bytes. Each named argument has a zero-based `u32 argument index`, byte **number of names** (current writer always 1), `u32 name length`, name bytes. The native reader can consume several names for an argument; this audit accepts the current writer's one-name form only. It also rejects legacy alternate prefixes and future format bytes. The native reader does not itself reject the top format byte; that restriction is the bounded current-writer policy. Raw floating point bits and string bytes are retained; no localization rendering, service behavior or Load→Save normalization is asserted.

CoastalLowland v9 is Lowland type hash, `i32 plot index`, starting-Terrain hash, six signed flow-direction bytes (E, SE, SW, W, NW, NE), river-crossing count byte, crossing-mask byte, six booleans (W/NW/NE/E/SE/SW river edges), flooded/submerged/protected booleans. Desert v6 and MountainRange v6 are definition hash, discovering-civilization hash, `i32 geographical Territory ID`, counted string8 custom name. NamedTerritory v1 uses a plain `i32 named-territory value` in place of the definition hash, then the same discovering-civilization hash, Territory ID and custom name. The six manager vectors retain their native order; source schemas do not justify inventing map-location or ownership meanings for Territory IDs.

These nonempty Route, Terrain and Territory schemas are source-derived support, not unknown payload retention and not restricted to the one native map's zero counts. Hand-authored structural fixtures exercised each nonempty decoder, every localized argument tag and truncation rejection. They are explicitly **not native differential fixtures**. Actual original-code nonempty serialization/Load fixtures and affected live-state comparison remain the acceptance gap. Historical record versions and unsupported variant branches still fail explicitly.

## Definition links and movement requirements

Terrain, Feature, Resource, Improvement, Continent, MapSize and Wonder Building wire values use their native type-hash serializers, not runtime collection ordinals or SQLite rowids. Resource/Yield/Improvement arrays similarly preserve definition hashes. Route type and route era are the explicit packed byte fields here. Definition import must use the exact active Linux rules registry, DLC/mod/configuration and native collection order, following [definition identity](linux-definition-ids.md); this audit does not manufacture registry indexes from hashes or database rows.

The typed Map supplies dense plot identity, topology, terrain/features/resources/improvements, cached impassability/hills, river/cliff masks, route state, owner/city links, unit occupancy, visibility, region/river/volcano data and map aggregates. Exact movement additionally needs native Unit/Formation state, definition costs/domains, player technologies/civics/abilities/modifiers, diplomacy/access, zones of control, reporting/cache obligations, pathfinder decisions and every affected state/RNG update. Omitted live fields and index caches need their actual native reconstruction paths. No movement legality, cost, transition, pathfinding or turn-equivalence claim follows from these codecs.

Reproduce without executing game code:

```sh
python3 tools/linux/map_state_audit.py \
  --module ../c6-assets-linux/libGameCore_XP2.so \
  --initial ../c6-ghidra/evidence/linux-game-load/canonical/ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884.bin.gz \
  --reloaded ../c6-ghidra/evidence/linux-game-load/canonical/85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8.bin.gz \
  --initial-trace ../c6-ghidra/evidence/linux-save-normalization/trace-captured-1.jsonl \
  --reloaded-trace ../c6-ghidra/evidence/linux-save-normalization/trace-captured-2.jsonl \
  --output /tmp/map-state.json \
  --private-details ../c6-ghidra/evidence/linux-map-state/decoded
```

Full decoded native values stay in private compressed JSON for comparison by the Rust implementation audit. Public evidence contains boundaries, identities, counts, summaries and limitations. The decoder supports the recovered current-version schemas above; native malformed-input behavior and arbitrary historical save compatibility are not asserted.
