# Trace-independent original Linux Core stream indexing

[`core_stream_index.py`](../tools/linux/core_stream_index.py) now finds and extracts the **complete Map section from raw Core byte zero**, using original serializer order, versions, counts and nested field framing. The normal path needs no native execution, owner trace, known stream hash, fixture offset, signature search or downstream player decoder. It reuses the structural field reader in [the Map contract](linux-map-state.md). This moves original world-byte import forward; it does not load a complete game or establish gameplay semantics for retained opaque payloads.

The grammar targets Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` with matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. The [checked report](../evidence/linux/core-stream-layout.json) pins 42 original serializer/reader method bodies, native version constants, real Rules virtual providers, parser sources and original validation inputs. The parser itself is not limited to those input hashes: it accepts its declared structural subset and rejects unsupported versions/providers or malformed bounds.

Prior work consulted before implementation: [Core normalization/owner evidence](linux-save-normalization.md), [Map fields](linux-map-state.md), [Unit/Formation codec](linux-unit-codec.md), [nested variant framing](linux-unit-wire-variants.md) and the existing host-container inspector. The pinned cru121 *Civ6-GameCore-reference* revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, especially `findings/topics/game-properties-storage.md` and the serializer/layout index, supplied discovery leads; CC BY 4.0 attribution and limitations are in [reference.md](reference.md). Field order here comes from the supplied Linux methods and existing Linux wire contracts, not old/current Windows offsets.

## Raw Core prefix through Map

This input is the uncompressed original **Core stream**, not a whole `CIV6` save container or simulator envelope. Existing extraction/container tools have separate version/profile limits and do not become general Linux save import through this tool.

| Order | Original wire grammar supported here |
|---|---|
| Context | `i32 version=5`, then unversioned TypedVariantMap metadata |
| Definition-index framing | `i32 IndexSerializer version=1` |
| Game | `i32 version=22` |
| Rules | `u32 version=1`, `u32 provider_count=1`, Placement provider hash `0xf3e6941e`, Placement version `1` |
| Game processor | `i32 version=2`; 64 uncounted i32 last-initial-AI entries; 64 uncounted i32 turn-order entries; i32 last-AI-processed turn; three bools: turn-complete, advance-turn, new-game |
| Player processor | `i32 version=2`; six uncounted 64-bool arrays; three uncounted 64-u32 update arrays; u32 active count; bool human-first-activated |
| Phase processor | `i32 version=3`; i32 current phase; u32 time limit; counted ordered `(u32 TurnSegment hash, bool disabled)` entries; i32 last-valid segment |
| Game scalars | i32 current turn, i32 max turns, i32 local player |
| Game properties | untyped VariantMap: counted `(u32 key,u32 bits)` scalars, then counted `(u32 key,ArrayVariant)` arrays |
| Map | complete Map v4 sequence, consumed structurally through its final named-component envelope |

All scalars are little endian; bools occupy one byte. **Native `Types::StaticArray<T,64>` has no wire count**, unlike `WriteArray` and the counted vectors used elsewhere. This distinction was checked in each actual int/bool/u32 static-array serializer. The Player arrays are activation-event, first-activation, activation-needed, automation-processed, player-complete, game-complete; then updates-this-phase, updates-this-turn and AI-updates-this-turn. Their serialized presence does not exclude AI-owned gameplay data.

These outer sections are generally **unframed**: there is no generic length table around Game, Rules, the processor records or Map. Rules version 1 currently writes only Placement, using actual manager virtual `+0x40`, Placement writer `+0x48`, and the ID returned by `+0x20`. Its payload is just ID and version. An unknown Rules provider/count is rejected; it cannot be skipped by a fabricated size.

Typed metadata follows the existing native keyed-map and ArrayVariant grammar. Type/modifier tags do not select payload length. An optional high-bit keyed header controls saved names; child variants have their own framing. `ValueIsArray 0x20000000` does not itself choose a writer branch. The indexer preserves framing without asserting native Load normalization or the semantics of arbitrary tags. Unsupported simultaneous child flags, recursive/resource bounds and overflowing raw-array lengths reject explicitly. Negative native map counts are outside this supported subset.

Map parsing reuses dimensions, arrays, visibility, recycled containers, Plots, occupancy and component-envelope framing from the Map reader. The indexer deliberately skips a named component's **explicit u32 payload length**, retaining its name/span/hash even if its payload schema is unknown. This is sufficient to locate Map's end. It establishes no gameplay behavior or exact reconstruction of that unknown component. The Rust Map decoder must separately accept and decode its supported component schemas; this indexer does not replace that decoder.

## Interface and extraction

```sh
python3 tools/linux/core_stream_index.py original-core.bin.gz \
  --output /tmp/core-index.json --extract-map /tmp/map.bin
```

Raw `.bin` input also works. Extraction refuses to overwrite an existing file; index output cannot overwrite the input. The Python API is `index_core(data)`: the `sections` entry named `map` gives `[start,end)`, byte length and SHA-256. Processor/component ranges may be nested; the section list is not a disjoint full-stream partition. Default parsing stops at Map's end and explicitly retains the untouched tail as unparsed. It needs neither Game.Religion nor any Player schema. Limits are a 256 MiB input, bounded counts, one million variant nodes and depth 64; these are supported-tool resource bounds, not claims about the native engine's maximum world size.

For the two original 80,657,462-byte normalization streams, independent parsing locates Map at `1884..4695864`, length **4,693,980**, SHA-256 `cd2ec75b4e342a6ebccc1b96889335352657b9ea67f83d5f6d56f6a94eaf2de9`, with 6,996 Plots. Those are **validation results**, absent from parser decisions. Existing native owner traces are consulted only after parsing to compare independently computed boundaries. Both extracted sections are then consumed to exact exhaustion by the standalone typed Map reader.

Synthetic framing tests add a metadata entry and replace empty Game properties with a larger scalar/array map; the parser relocates Map by the encoded lengths while preserving its exact payload hash. Another test adds an unknown but length-framed Map component and checks its span. These exercise offset independence and permitted opaque framing; they are not new native fixtures or gameplay acceptance. Truncated metadata/Map and unsupported Context version reject. Reproduce the module/source/fixture checks with:

```sh
python3 tools/linux/core_stream_index.py audit \
  --output target/linux/core-stream-layout.json
```

## Formation extension and the precise Unit stop

`--first-formations` optionally extends the cursor after Map. It closes Game.Religion v7, including each ReligionData v7 record: religion hash/player/founded turn, PlotCoord, counted name bytes, three counted definition-hash vectors, int, three bools and final int. Next comes Player.Manager v1's player count and first explicit player ID, then the first Player v47 scalar prefix. That prefix contains its own player ID, starting PlotCoord, fifteen i32/enum scalars through EndTurnBlocking, six bools, then Handicap/District/Project hashes and an int. Matching outer/inner player IDs are required.

Player.Formations v1 contains a normal `RecycledIDContainer` of Formation v1 records. Record fields follow the existing codec: fixed 36-byte prefix including version, counted 20-byte path nodes, cached coordinate, counted member-unit IDs. Container slots/free-list metadata are parsed, not searched. On each original fixture this finds the first player's **35 Formation records**, ending at 4,698,763; every computed record boundary matches the independent native ownership evidence. It is not the full 1,250-Formation roster, and it does not index Unit records.

The next exact unframed serializer is **Player.Goody_Hut**. Between this point and Player.Units, the Player writer invokes Goody_Hut, Bonuses, Congress, Cities, Diplomacy, Districts, NameManager, Culture, Espionage, Religion, Resources, Stats, Techs and Treasury, in that order. Player records themselves have no length prefix. After Units there is further player state before the next player's record. No safe generic skip or byte-signature lookup follows from existing per-owner Unit/Formation boundaries.

The native ownership/codec evidence already provides 1,262 exact Unit records and 1,250 Formation records across the fixture, with real stream hashes and ranges; it remains useful for controlled component export and codec validation. Those recorded offsets are not a library parser. A future complete-roster raw-save index needs the intervening unframed schemas, or a separately faithful native component export at a supported boundary. This does not block importing the independently indexed complete Map now. No new general export framework is introduced here.

Complete Core/Unit/Formation-roster index flags, full world import and gameplay parity remain false. Unknown tail bytes remain untouched and unmasked; the 89,198-byte original Save→Load difference is preserved. Exact logical gameplay state, RNG, initialization, actions, turns and all external player decisions remain the overall objective.

## Current Player component extension

The optional `--first-player-components` flag (Python `first_player_components=True`) extends the first-Formation path through complete current Goody_Hut v1, Bonuses v4 and Congress v12, including counted nested Refund records. The new structural stop is Player.Cities; the default Map-only and old Formation-only modes keep their original stops. The production Rust codecs, exact Reset versus overlay rules, source/native-stream audit and observed City boundary are documented in [Player components](linux-player-components.md) and [their separate checked report](../evidence/linux/player-components.json). This extension does not locate a full Unit roster or decode complete City/Player state.

## First City prefix extension

`--first-city` (Python `first_city=True`) implies the first-player-components mode and follows Player.Cities v15 plus recycled-container v65537 framing into the first real City slot. It parses the complete current City51 pre-component header, Districts4 and Citizens2 and stops at Culture. Empty first-player City containers stop explicitly at their unparsed tail. The separate complete Rust City record codec and original-byte evidence are described in [City wire import](linux-city-wire.md); this partial Python mode is not a complete City/Player/Unit index. A Rust byte-0 Core reader will compose these prefix APIs directly rather than require Python offsets.

## Direct library entry from byte 0

The separate [Rust Core prefix API](linux-core-prefix.md) now composes the same supported current prefix and existing Map parser directly from raw byte 0. It returns typed data, complete Map, consumed cursor and a borrowed untouched tail; library callers do not need this Python tool's offsets or traces. Its separate [checked profile](../evidence/linux/core-prefix.json) compares both original raw streams and length-changing source-only carriers. This remains a partial import ending before Game.Religion, not complete executable world state.

The newer Rust [`ThroughFirstPlayer`](linux-core-player-prefix.md#direct-byte-zero-library-composition) API continues from byte zero through Game.Religion and the first Player's complete Formation, Goody_Hut, Bonuses, Congress and Cities collections. It computes absolute stop 4,907,389 on both existing originals, before Player.Diplomacy; this does not complete any Player entry or the full roster. Its separate [wire audit](../evidence/linux/core-player-prefix.json) retains complete raw differences and an untouched borrowed tail.

The separate [current Districts-stage Rust profile](linux-core-player-districts-prefix.md) continues through full Diplomacy and Districts, with dynamic stop 4,988,302 before NameManager on both existing originals. Its complete post-Map region roundtrips exactly; no Player entry is finished, and NameManager/remaining Player schemas are not skipped. Earlier accepted profile artifacts remain immutable.

The [Culture-stage Rust profile](linux-core-player-culture-prefix.md) continues through NameManager2 and Culture45 to dynamic cursor 4,995,296 / first Player.Espionage on both originals. NameManager double counts, acquired civic pairs and saved IsAlive are independently projected; all earlier accepted profiles remain immutable. This still does not complete any Player entry or reconstruct a complete executable state.
