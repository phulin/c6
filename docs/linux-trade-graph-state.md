# Owned TradeGraph path queries

`trade_graph_state::GraphState` owns one complete decoded Graph and supports
read-only point path queries. `LoadedState` admits it once from an explicitly
attributed source frame. This supports the reporting warm-path dependency;
it does not implement Trade rules, active-route callback order, pathfinding,
complete native loading, actions or turns.

The existing [codec](linux-trade-codec.md) and
[Read/lookup contracts](linux-trade-serialization-contracts.md) provide the
stored keys, edges and paths. No new writer, broad prefix reader or captured
answer map is introduced. The existing source-derived Vertex hash is reused
for the inner table's unsupported reserved-hash guard. Duplicate keys/IDs,
noncanonical fields and endpoint mismatch remain explicit codec errors.

Native GetEdge uses outer then inner key lookup. Inner Find compares the hash,
then full Vertex equality, continuing across collisions until an empty slot.
The owned logical indexes preserve that result without native bucket or pool
layout; they never control execution iteration. Distinct equal-hash vertices
remain distinct. The native inner empty-slot hash `0xffffffff` cannot be an
admitted destination key; an empty outer origin with that hash remains allowed.
The complete relevant native bodies and selected instructions are fingerprinted
by `tools/linux/trade_graph_state_check.py`; raw disassembly remains private.

CityID conversion at `0x2bf85f0` creates Vertex(kind0, signed owner, full ID bits).
Component type does not enter this key; low16 recycled-slot identity cannot
replace the full ID. `get_path` / `city_path` reproduce Graph.GetPath's null
result for an absent edge or empty path. Active-route flags, traversal
cursor/direction and the Edge's stored Route endpoints do not filter this
lookup. Callers query the actual supplied city IDs, which need not equal the
currently iterated Edge's outer keys.

`reverse_city_path` performs the swapped Graph lookup and exposes every stored
plot index in reverse order, including repeats, without editing the Graph.
This is the warm reverse-copy projection inside native FindNewPath, not that
method's earlier Map/GetCity checks. The gameplay adapter must retain those
checks. GetPath itself may return a native null result; a full Trade calculation
that needs a path must explicitly reject the forward-plus-reverse miss until
the real finder/working state/callback/RNG branch is implemented. No cache fill
or successful cold-path result is fabricated.

## Admission and lifetime

`install_trade_graph(witness, frame_sha256, limits)` requires pristine state,
no prior owner, SavedOutput source role, the exact full-image identity/length,
a nonzero trace witness, bounded nonempty owner range, exact frame length and
matching frame SHA. Decode and index construction complete before installation;
failures leave the owner/source absent. The data adapter verifies the actual
serializer trace, frame attribution and module/resource identity. This Core
boundary checks consistency and exact bytes; caller assertions alone are not
execution authentication.

Queries borrow the owned Graph with no per-query hashing or global revision
gate. Unrelated state writes preserve this authority. No mutable Graph reference
is exposed. Any future operation that changes native Trade graph data must
apply the exact owned changes or reject before reporting success; it cannot
silently continue with an initial snapshot. Current reporting domain guards
must reject unsupported callbacks/effects. This is a source-rebuilt logical
index, not an observation that the original live hash tables were consistent.
Native Graph.Read's preexisting-container behavior is not reproduced here.

The Graph view retains decoded wire state for domain checks. Omitted live Route
fields, definition-remapped yield storage, native ID-map callback iteration,
and complete Trade Manager state are separate dependencies. In particular,
Graph path admission does not establish Monitor cache freshness, values or
post-initialization history. No new native execution was performed.

## Validation

The initial standalone report [trade-graph-state.json](../evidence/linux/trade-graph-state.json)
retains complete compiled source preimages, including its explicitly recorded
new-file WIP, in a private read-only source archive. The admitted input is the
known ff31 Core's Graph frame `[71027800,71497438)`, SHA
`64a196d77b6bc35a72354226dda4111e10f46742cd62d4e335b43c737a186c33`.
Its original trace SHA is
`528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae`,
save1 Graph owner call162111/parent162110/depth2. The tool selects that exact
frame, without scanning unknown Core bytes for a plausible Graph.

All1,571 keyed paths are nonempty in this frame. All182 active Routes' actual
city-key lookups succeed, covering6,193 nodes;182 reverse projections preserve
exact node order. These are Rust evaluations of attributed original input,
not direct native getter comparisons. Five focused cases discriminate signed
full-ID keys/component types, forward versus reverse, empty/missing paths,
full-key hash collisions, sentinel keys and exact frame consumption.
Two bridge cases test atomic admission rejection, replay/late refusal and
continued ownership across an unrelated Formation mutation. Strict Core
all-target Clippy passes. The private source-only carrier has no Unit records;
actual loaded-state Unit mutation coverage belongs to the data integration
check rather than a fabricated Unit fixture.

`--source-commit COMMIT` also archives the complete committed tree and runs the
seven targeted module/bridge cases from that retained snapshot. Complete source,
exact executables, Cargo.lock/toolchain identities and logs are retained. The
raw historical89,198-byte Save→Load failure remains unclassified and unmasked;
no Save-equality acceptance gate is introduced.

The [committed-tree report](../evidence/linux/trade-graph-state-1038c8f.json)
independently builds source `1038c8ff1cec070f5ee771db55392c1caca87527` and passes
all seven targeted module/bridge cases. Its complete retained archive SHA is
`a4ce889bed979a59d3469700f1d5c12a688ea79ea6b6fe81272002787504db3b`;
all1,698 committed source preimages were checked after building and made
read-only. The Core test executable SHA is
`4a6cdea76611f1f31ec9bde6c3580aab0c00f31b2aa8502a8a56b15df25cb81b`.
The frame checker executable is
`9a868b65c5aaf2df68f5d8727d45fbfa35a6a22727c952ff2f920270c92bed47`.
The checker driver's then-uncommitted source is explicitly recorded and fully
retained alongside the committed production tree. Native query comparison and
live index observations remain false in this report.
