# Independent Linux Player.Trade wire review

The current bounded Trade v35 codec passes this focused review at committed
source `e7f2aab03db7473a4e9c5b5d29b9ab7c42678114`. No implementation defect
was found. This accepts the individual wire boundary; the expanded byte-zero
Core prefix through Trade has its own producer and composition review.

The [review script](../evidence/linux/player-trade-review.py) and
[closed report](../evidence/linux/player-trade-review.json) reproduce the result:

```sh
python3 evidence/linux/player-trade-review.py \
  --output evidence/linux/player-trade-review.json
```

One immutable Git archive supplies the implementation, public export and all
dependencies. The report pins its full source commit/archive hash, focused
source hashes, generated harness and actual executable identity. Build paths
can affect executable hashes; each rerun records a fresh identity. No working
tree source is recovered into the archive. Original game code is not executed,
and no private artifact is written.

The hash-gated Linux Core is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The review rechecks all 54 full method extents, symbol identities, 617 concrete
PLT bindings, matching Trade DWARF and current version 35 at `0x2f68bbc`.
It verifies Player's Trade call at `0x2b4f11f` and following GreatPeoplePoints
call at `0x2b4f132`. Source reference names remain discovery leads; target
Linux instructions and native saved records supply acceptance.

Writer `0x2b92910` and reader `0x2b92f00` establish the grammar. After the
version come one PlayerTypes i32 and fifteen signed scalar fields, then two
u32-counted byte-bool vectors. Thirteen Yield hash/Q8 arrays precede the signed
techs/civics-per-yield value and eight more arrays. The next field is exactly
64 consecutive per-player Yield tables with no outer count or player hashes.
Three Improvement hash/nested-Yield arrays follow, then the international
specialty-district Yield table, four Terrain hash/nested-Yield arrays and
three signed trade-disabled counters. There are no strings, localized labels,
route records, diplomatic-deal pairs, nested version words or serialized
packed ComponentID in this grammar.

The cardinality 64 comes from the initialized native invariant: Reset resizes
the per-player vector using literal `0x40` at `0x2b95854`, and the writer
iterates its entries at `0x2b92d80`. The codec models that initialized current
wire grammar; it does not infer an outer count or reproduce arbitrary
uninitialized native objects. Each inner table retains its own unsigned count.
An outer nested entry is a raw definition hash followed immediately by a
complete Yield table, without a length envelope or second version.

Yield, Improvement and Terrain identities are raw unsigned definition hashes,
not SQL row positions or live indexes. Q8 values remain signed raw i32 bits.
Ordered pairs, duplicates, unknown hashes, empty tables and negative values
survive without sorting, clamping or lookup normalization. The saved player
is a raw PlayerTypes ordinal, including negative bit patterns; it provides no
independent authority to resolve a live Player or establish relationships.
The bool vectors retain their positions and declared lengths. Their wire
contains no per-entry player or definition identity.

Native Read uses the saved PlayerTypes value in Globals::GetPlayer and calls
Initialize at `0x2b92f42`. Initialize sets the parent, calls Reset at
`0x2b93e36`, then assigns archive ownership. For participant players it constructs
a live Player::ComponentID with component number zero and type `0x23`, calls
GetBits and passes that packed result to FAutoArchive::SetID; the nonparticipant
branch supplies its native sentinel. Those are native initialization operations,
not additional bytes in the Trade record. The codec preserves the saved i32
player token; it neither repacks it nor claims the live archive-owner result.
Native definition overlays, archive edits, callbacks and any transitive RNG or
gameplay effects remain unimplemented. This component is separate from global
trade managers and Unit.MakeTradeRoute operation transport.

Both original canonical streams supply 43 complete records each, totaling
1,340,224 bytes. Every record is 15,584 bytes with 2,066 charged items. Each
passes exact decode/re-encode bytes and item counts, every truncated prefix,
exact versus one-short byte/item budgets in both directions, strict trailing
rejection and an uncharged one-MiB tail under exact prefix limits. Owner traces
select offline validation cuts only; production parsing receives no trace
metadata or saved offsets. Repeated component shapes across streams do not
establish new native branch coverage or a native mutation oracle.

An independent Python interpreter walks the declared wire grammar, deriving
count boundaries and item algebra. A Rust projection extracts every typed
scalar, count, boolean, hash and raw Q8 value in that reviewed field order.
Their complete token sequences match for all 86 native records. The same check
passes a manually framed synthetic record with distinct signed scalars,
true/false values, duplicate/high-bit hashes, Q8 extrema, nonempty and empty
nested tables and varying per-player table lengths. Another 118 mutations
exercise unsupported versions, every dynamic count with a hostile value and
each boolean with a noncanonical byte. They fail under huge caller limits,
so tiny payloads cannot request their declared large allocations before wire
preflight. The two committed source boundary tests also pass. Synthetic
branches provide source coverage, not comparisons to native mutated state.

Item charges include every boolean, every Yield pair, every outer nested pair
and all 64 fixed player slots. Fixed scalar fields and count words charge only
bytes. Each dynamic read preflights aggregate items, checked minimum wire bytes
and available input before fallible reservation; reservation also checks typed
allocation addressability. Typed nested entries can exceed their serialized
minimum size, so wire bytes and logical typed storage remain separate policies.
Encoding measures the entire output and aggregate item count before a fallible
output reservation. No total heap-byte cap, allocator-overhead bound or runtime
allocation equivalence is claimed.

Complete 43-Player loading, full gameplay state, all gameplay RNG, actions and
turns remain unfinished. Player decisions remain external policies. Unknown
save bytes remain unclassified and unmasked, including the recorded
89,198-byte raw Save→Load difference. GreatPeoplePoints is the next component.
