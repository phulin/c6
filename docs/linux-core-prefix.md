# Direct Rust raw Core prefix import

`c6_core::core_stream::CorePrefix::decode` now reads from **byte 0 of a raw original Core stream** through its complete typed Map. It composes existing Rust variant and Map codecs and reports the consumed cursor with a borrowed untouched tail. Library callers need neither Python-generated offsets nor native traces. This is an explicit partial import, not complete executable game state or native Load equivalence.

The [source module](../crates/c6-core/src/core_stream.rs), [reproducible audit](../tools/linux/core_prefix_audit.py) and [checked report](../evidence/linux/core-prefix.json) are scoped to original Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` with matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. Both identities are independently checked against the supplied original ELF files. The 42 original method fingerprints and existing [Core grammar](linux-core-stream-layout.md) establish framing. The pinned cru121 revision `5af389445118bed2bdebf7a8a42b219c8fba5175` supplied discovery leads under CC BY 4.0; [attribution and limits](reference.md) apply. No Windows identity, relative build age or SQL ordinal assumptions establish this support.

## Typed prefix and explicit boundary

The reader consumes:

1. Context v5 and TypedVariantMap metadata.
2. IndexSerializer v1, Game v22, Rules v1 with the original Placement provider v1.
3. GameProcessor v2, PlayerProcessor v2 and PhaseProcessor v3.
4. Current-turn, maximum-turn and local-player scalars; the untyped Game properties map.
5. Complete Map through its named component envelope, using the existing supported [Map codec](linux-map-state.md).

Unknown **unframed** Rules provider counts/hashes are refused rather than skipped. Supported current Rules has one Placement provider, hash `f3e6941e`. All version and nested variant/component gates remain explicit. This wire profile does not imply arbitrary scenario/provider support.

The typed result contains metadata, both processors, phase state, Game scalars/properties and Map, followed by:

- `map_start`, `map_end`: dynamically computed byte range.
- `consumed`: the exact prefix end, equal to `map_end`.
- `tail: &[u8]`: a borrow of the original input starting at that end.

The next real unparsed section is **Game.Religion**. Player/Unit/City rosters, later global state and RNG have not been imported by this API. The separate [complete City wire codec](linux-city-wire.md) is ready for later composition, but this API does not jump over intervening unframed sections to reach it. Unknown tail bytes are neither removed nor treated as semantically irrelevant.

```rust
use c6_core::{core_stream::CorePrefix, map::Limits};

let prefix = CorePrefix::decode(&raw_core, Limits::default())?;
let map = &prefix.map;
let remainder = prefix.tail; // original input borrow; not an opaque simulation state
```

Input is the decompressed raw Core stream, not the host `.Civ6Save` container or gzip transport. The audit decompresses its existing fixtures only to provide that raw buffer; the Rust parser performs the entire structural decode itself.

## Values, limits and strict-codec reuse

The processor arrays are original **uncounted 64-element arrays**, not wire vectors. Both AI-labelled int arrays, AI update counts and every serialized bool remain represented. The three GameProcessor bool values are original **namespace globals**, serialized in this exact order: `GameCore::Game::ms_bTurnComplete` (`0x3c05cb8`), `ms_bAdvanceTurn` (`0x3c05cb9`), `ms_bNewGame` (`0x3c05cba`). Target writer `0x2a0b1c0` and reader `0x2a0b120` agree; they are not guessed Processor object offsets. The selected current reader includes NewGame for version >=2.

`map::Limits` supplies an aggregate consumed-byte/item/allocation/depth policy. Consumed bytes exclude the borrowed tail, so the default 64 MiB prefix budget accepts the existing 80,657,462-byte Core inputs without copying their 75,961,598-byte tails. Fixed processor-array elements, phase entries, variants and Map data share the item budget. Logical owned allocation is bounded with conservative variant charges; native allocator addresses/capacity/rounding are not recreated. No full world/tail clone is used to find Map.

The approved `Map::decode_prefix_with_items` wrapper returns consumed bytes and the existing parser's item charge. It reuses the same parser body and preserves `Map::decode` and `decode_with_ranges` strict exhaustion and original budget behavior. Meaningful tests check longer carriers, exact prefix bytes, strict trailing refusal and cumulative budgets. The original complete 4,693,980-byte Map strict roundtrip was rerun after the refactor.

Metadata is retained as typed wire data. The reader does **not** infer a producer module identity, validate Context GUID/version compatibility, restore native globals, run reporting/registration callbacks or apply native Read fixups. Callers must separately gate original provenance and active rules/DLC/mod configuration before using imported values for supported gameplay. No UI/native AI policy computation is invoked, and no AI-namespaced data is blanket-excluded.

## Original-byte evidence

The audit runs the separate offline Rust driver on both immutable original 80,657,462-byte raw Core streams. It independently compares metadata, all processor/phase/scalar/property fields and computed boundaries to the source-derived Python oracle. The driver receives the **entire raw Core**, never a pre-extracted Map or offset. Re-encoding the returned typed Map must equal the corresponding original byte range. The result's tail pointer must refer to the original buffer, and the complete tail digest is compared without masking.

Both original fixtures produce:

| Value | Observed result |
|---|---:|
| Map start | 1,884 |
| Consumed/Map end | 4,695,864 |
| Complete Map bytes | 4,693,980 |
| Dimensions/plots | 106 × 66 / 6,996 |
| Borrowed unparsed tail bytes | 75,961,598 |

Map SHA-256 is `cd2ec75b4e342a6ebccc1b96889335352657b9ea67f83d5f6d56f6a94eaf2de9` on both. These positions are outputs, not parser constants. Source-only cases independently add an extra framed metadata entry and replace Game properties with a longer supported scalar/array map; Rust relocates Map by the actual 20-byte and 24-byte deltas while preserving Map bytes. This does not claim those extra fields were dynamically produced by the native fixture.

The two original full streams still differ in **89,198 raw bytes**. The difference is reported unmasked; equal prefix/Map observations are not a whole-state or RNG exclusion. Metadata compatibility, complete Core/Player import, RNG reconstruction and gameplay parity remain false in the report.

Reproduce with:

```sh
python3 tools/linux/core_prefix_audit.py --output target/linux/core-prefix.json
cargo test -p c6-core core_stream
```

This is original **wire-data comparison**, with an offline Rust candidate and exact source/binary identities. It performs no original-code execution, native rerun, Ghidra/private repository change or upload, and leaves all frozen movement/City profiles intact. Full state/action/turn support still requires the subsequent real schemas, live read-derived gameplay state, all affected RNG and genuine external policy decision boundaries.

For continued readers, `Map::decode_prefix_with_usage` exposes both the existing parser item charge and logical allocation charge in addition to consumed bytes. The initial `decode_prefix_with_items` API remains available. Exact and one-short allocation limits are tested; these are conservative Rust resource-policy charges, not native heap/layout state. Historical evidence above retains its immutable tested source identities.
