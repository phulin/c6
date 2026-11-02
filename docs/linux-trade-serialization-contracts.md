# Linux trade serialization contracts

These are bounded contracts checked against supplied Linux GameCore SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, its dynamic symbols, actual bytes and DWARF. They describe original trade wire data and selected consumers, not complete game-state/action/turn support. Trade is gameplay state and cannot be excluded because related code uses an AI namespace.

`python3 tools/linux/trade_serialization_audit.py --output evidence/linux/trade-serialization-contracts.json` verifies the module identity, 26 exact method boundaries/byte hashes and current version assignments. Private persistent Ghidra evidence is `../c6-ghidra/evidence/linux/ghidra-trade-*.json`; actual instruction listings and target DWARF are under private `evidence/linux-save-normalization/`. No Windows layouts or behavior establish these contracts.

## Exact boundaries and current wire

| Type | Write | Read | Current version | Native size |
| --- | --- | --- | --- | --- |
| Graph | `0x2bf7fe0` | `0x2bf8210` | 1 | 352 |
| Route | `0x2c113f0` | `0x2c114b0` | 4 | 112 |
| Edge | `0x2bf8150` | `0x2bf83c0` | 3 | 536 |
| Vertex | `0x2bf7f10` | `0x2bf7f90` | 1 | 12 |
| Manager | `0x2c050b0` | `0x2c05100` | Version word consumed separately | 640 |

All words below use the target's little-endian encoding. Native `FAutoVariable` wrappers are not copied as object bytes: the original virtual serializers write their values. Preserve the wire version and deterministic order; do not write object padding, pointers, archive owners or allocation fields into this format.

A `Vertex` writes four signed 32-bit words: version 1, type, data1, data2. Type is sign-extended from a native signed 16-bit enum; its actual values are `NO_VERTEX=-1`, `VERTEX_CITY=0`, `VERTEX_UNIT=1`, `VERTEX_PLOT=2`. The native reader narrows the type word back to 16 bits. Unsupported malformed/high-bit encodings require separate boundary fixtures; a current fixture decoder must not silently normalize them.

A player `CityID` or `UnitID` writes signed 16-bit owner, signed 16-bit component type, then unsigned 32-bit ID. `ComponentID::Set` reconstructs the live identity on read. Treating these eight bytes as two unnamed integers obscures the actual identity fields.

A current `Route` writes version 4, then:

1. `m_iID` (i32), `m_iStartTurn` (i32), `m_kTraderUnit` (ComponentID).
2. `m_eResourceType` (u32 definition type hash), `m_iResourceQuantity` (i32).
3. `m_Origin`, then `m_Destination` (ComponentIDs).
4. `m_axOriginYields`, then `m_axDestinationYields` (hash-index arrays).

Each yield array writes a u32 current-definition count, followed in current definition order by pairs of u32 type hash and signed i32 `FixedPointT<8>` raw value. This is not floating-point conversion. Actual `WriteIndexHashArray`/`ReadIndexHashArray` are `0x1ff74c0`/`0x1ff7160`. Read maps each hash to the current definition index; for an unknown hash it still consumes the value into a temporary. Definition identities, order and values must therefore remain explicit. Under the current six-yield fixture each array is 52 bytes and the complete Route is 148 bytes.

`m_bFinalizing` at native Route offset 104 is omitted by both wire operators. Equality of these serialized fields does not prove equality of all live Route state. The Route reader also contains legacy `<3` branches consuming obsolete arrays, but the strict current-version contract does not claim legacy support.

A current `Edge` writes version 3, then values in this exact order:

| Field | Native wrapper offset | Wire value |
| --- | --- | --- |
| `m_iID` | 152 | i32 |
| `m_kMostRecentRoute` | 176 | Route |
| `m_bHasActiveRoute` | 304 | one byte |
| `m_iRoutesStarted` | 328 | i32 |
| `m_iRoutesCompleted` | 352 | i32 |
| `m_vMostRecentPath` | 376 | u32 count, then i32 plot indices in path order |
| `m_iPathTraversalIndex` | 424 | i32 |
| `m_bPathTraversalGoingForward` | 448 | one byte |
| `m_kFromVertex` | 472 | Vertex |
| `m_kToVertex` | 504 | Vertex |

The path count writer `0x2045690` proves the four-byte count and absence of a separate vector version word. Preserve path order, raw fixed-point values and flag bytes; the current fixture decoder explicitly refuses unsupported noncanonical bool bytes. Edge version 2 additionally reads/discards a legacy vector after the path; that branch is not current-version support.

## Graph loading and ordering

Graph Write emits version 1, the value of `m_iTotalEdgesCreated`, and signed i32 origin count. It traverses `m_kOriginMap` in its native EASTL hash iteration order. For each origin it writes the origin Vertex, signed i32 destination count, then each destination Vertex and complete Edge in the inner `Types::HashMap` iteration order. It does not sort edge IDs, write the block allocator, or serialize the reverse destination index separately.

Graph Read reads this sequence and, for each edge, allocates from the real block allocator, runs Edge Read, attaches its auto archive, and uses the preserved Edge ID to populate the ID map. It inserts the edge into the origin and reverse destination indices using the decoded outer/inner vertices. There is no direct edge-ID renumbering in this reader. The bounded reader itself does not clear pre-existing graph containers; correct native object ownership and load initialization remain necessary.

Manager Write emits its version, Graph, then `m_iTotalRoutesCreated`. Manager Read consumes its version and delegates Graph and counter reads. Its temporary route vector, message handles, graph job state and component owner pointer are not serialized by these operators. Manager Initialize at `0x2c05590` is a separate lifecycle boundary; its presence does not establish a complete initialized turn subsystem.

Original `Graph::Validate` at `0x2bfc690` checks ID-map size against allocated-edge count; preserved ID lookup; both forward and reverse vertex mappings back to each identical Edge; and inner-map count totals. A typed decoder should retain outer origin membership, including empty origins, reject duplicate keys/IDs in its supported fixture, and compare outer/inner keys with the actual Edge from/to fields. It must not repair disagreement or discard duplicates to force equality.

`Graph::GetEdge(Vertex,Vertex)` at `0x2bfc240` performs an origin-key lookup followed by an inner destination-key lookup. `Graph::IterateRoutes` at `0x2bfa860` walks the ID map and invokes the caller callback for active routes. `IterateOutgoingRoutes` at `0x2bfa910` similarly traverses an inner map. Thus reconstruction can change serialized traversal order while preserving all keyed values, and traversal order is observable by consumers. Keyed equality alone does not prove future callback order, tie-breaking, RNG consumption or turn equivalence. Preserve observed iteration order separately from keyed comparison; do not canonicalize simulator execution into sorted order.

## Native comparison boundary

The forwarding-only original Save trace attributes 89,156 of 89,198 changed bytes in the current 80,657,462-byte Save/reload/Save pair to trade: 40,586 Graph-owned bytes and 48,570 Route-owned bytes. The latter are 522 changed Route occurrences, not the full graph's edge count. Graph occupies `[71027800,71497438)`; the first actual Route occupies `[71027856,71028004)`, matching the 148-byte current contract.

The separate strict fixture decoder in `tools/linux/save_normalization_trade.py` compares complete keyed payloads and preserves origin/destination iteration order. Its native owner evidence is [the typed trade comparison](../evidence/linux/save-normalization-trade-values-1.json). Reported fixture counts are 179 origins, 1,571 edges and `totalEdgesCreated=1571`; the decoded endpoint/path/yield/ID/flag payloads compare equal while recorded iteration order changes. The current Save hashes and every native Route boundary remain part of that separate gate. This establishes a bounded serialization-order difference, not permission to drop trade data or claim gameplay equivalence. The other 42 outer GameInstance bytes remain outside this contract.
