# Original Linux Save roundtrip differences

The c6 Rust implementation still covers isolated primitives. Original native
benchmark Core Load and repeatable Save now succeed, but the lower diagnostic
Save→Load→Save comparison has a reproducible byte difference. This investigation
must identify actual serialized fields and their native read/write behavior;
unknown bytes are not normalized or masked.

The current required scope is exact gameplay state and gameplay RNG, with external
policies choosing every player's decisions. Proven native AI planning internals
are outside that scope. An AI namespace or manager name alone does not prove an
exclusion: gameplay-affecting data and RNG stored in those classes remain required.
Until specific fields and uses are established, changed data stays unclassified.

The canonical private native outputs are both 80,657,462 bytes:

| Lower diagnostic input | Original output SHA-256 |
| --- | --- |
| Benchmark extracted Core stream | `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884` |
| Captured output from the first row | `85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8` |

Each result repeats across independent processes with successful original Load,
complete input consumption, two identical consecutive Saves and complete native
provider cleanup. The canonical outputs differ in 89,198 bytes across 31,695
spans. [The retained failing case](../evidence/linux/game-load-roundtrip-differences-1.json)
records their ranges. Equal coarse native readouts—turn 592, phase 0, local player
0 and false processing/pending predicates—do not establish equality of their
other state. The existing fixture still omits saved RequestLoad/Begin and complete
host postload finalization; no cause is attributed to those omissions yet.

The observational profile maps original serializer owner entry/exit offsets.
[The static audit](../evidence/linux/save-normalization-trace-contracts.json)
checks 207 exact XP2 exported owner methods against the pinned Linux module and
matching DWARF. Every selected interface returns its original OStream reference
and takes the original OStream and const owner references. Forwarding wrappers
call the original method and read the genuine MemoryOStream offset before and
after it. They change no stream fields, vtables or game fields. Call-parent IDs
separate repeated owner instances; raw traces remain private.

Observed ranges are usable only after the instrumented native profile reproduces
the known uninstrumented output hash and native cleanup. Owner boundaries alone
prove neither field semantics nor normalization. A classification requires the
specific original serializer field, its original Load handling, and the use that
shows whether it is gameplay/RNG state or excluded policy planning state. Logical
state equivalence, branch restoration and external decision readiness remain
separate acceptance requirements.

Both initial forwarding profiles now pass native Load/Save and complete cleanup,
while preserving both known original Save fingerprints exactly:
[benchmark trace](../evidence/linux/save-normalization-native-benchmark-1.json)
and [captured-input trace](../evidence/linux/save-normalization-native-captured-1.json).
The original calls and real stream behavior remain in place. Complete traces and
per-call mappings are private and hash-anchored by their public reports.

[The benchmark owner map](../evidence/linux/save-normalization-owner-map-benchmark-1.json)
and [captured owner map](../evidence/linux/save-normalization-owner-map-captured-1.json)
locate **89,156 of the 89,198 changed bytes inside Trade::Graph**, under the native
Trade::Manager serializer. The graph spans stream offsets 71,027,800–71,497,438.
Differences occur both within 522 serialized Trade::Route records and elsewhere
in the graph. Individual record boundaries differ between the two profiles; each
mapping therefore uses its own actual native trace. The remaining 42 changed
bytes are currently covered only by the outer Game::Instance boundary and require
more specific owner evidence.

Trade gameplay data cannot be excluded as AI planning internals. Exact Graph/Route
read/write code and matching DWARF identify native hash-container iteration and
rebuilding as a concrete ordering lead. A keyed comparison of all actual edge,
vertex, route, yield and path values is still required, together with iteration
and tie-break use evidence. Neither logical equality nor harmless encoding order
has been established from these owner ranges.

The reviewed fixture decoder
[save_normalization_trade.py](../tools/linux/save_normalization_trade.py) now
compares every serialized Trade graph field, checks original origin/destination
index invariants and rejects duplicate outer origins or edge keys. It retains
empty origin buckets and all original wire sequences. Both graphs contain 179
origins, 1,571 uniquely identified edges and totalEdgesCreated 1,571. No keyed
edge payload or origin membership changes. All 89,156 Trade-region byte differences
are therefore record-order differences in these fixtures: 54 outer origin wire
positions, destination order for 18 origins and 372 edge-ID wire positions change.
[The typed result](../evidence/linux/save-normalization-trade-values-1.json)
anchors the complete private values and both native traces. These ordering changes
remain recorded state; no future action or ordered-callback equivalence is claimed.

The current little-endian native wire contract for a typed codec is:

| Record | Fields in original serialization order |
| --- | --- |
| Graph v1 | i32 version, i32 totalEdgesCreated, i32 originCount; each origin Vertex, i32 destinationCount; each destination Vertex, Edge |
| Vertex v1 | i32 version; i32 sign-extended native i16 type; i32 data1; i32 data2 |
| ComponentID | i16 owner; i16 component type; u32 ID |
| Route v4 | i32 version, i32 ID, i32 startTurn, trader ComponentID, u32 resource type hash, i32 resource quantity, origin/destination City ComponentIDs, origin/destination Yield arrays |
| Yield array | u32 count; pairs of u32 yield type hash and i32 FixedPoint8 raw value; six definitions in this fixture |
| Edge v3 | i32 version, i32 ID, mostRecentRoute, u8 hasActiveRoute, i32 routesStarted/completed, u32 path count and ordered i32 path elements, i32 pathTraversalIndex, u8 pathTraversalGoingForward, from/to Vertices |

Native bool fields are one byte; the decoder accepts only zero or one. Vertex type
values are NO_VERTEX=-1, CITY=0, UNIT=1 and PLOT=2. Fixed-point values remain raw
integers. Route's live `m_bFinalizing` is omitted from this wire format. The graph's
reconstructed indices, their runtime iteration order, jobs and sync/archive state
are not made complete merely by decoding these records. This codec contract is
separate from loading a complete Civ VI save or advancing gameplay.

The expanded 207-owner profile adds the four original GameEffects serialization
interfaces. Its [benchmark](../evidence/linux/save-normalization-native-benchmark-2.json)
and [captured-input](../evidence/linux/save-normalization-native-captured-2.json)
runs retain the exact known Save hashes and complete cleanup. The
[expanded owner map](../evidence/linux/save-normalization-owner-map-benchmark-2.json)
locates the remaining 42 changed bytes: 14 within GameEffects::Instance and 28
within GameEffects::RequirementSystem. Their field identities and native Load
handling require their own evidence; owner names alone do not justify exclusion.

The original trace also observes one Game-owned synchronous RNG serialization
per Save, with the complete 18-byte record at offsets 71,027,778–71,027,796.
[The RNG comparison](../evidence/linux/save-normalization-serialized-rng.json)
checks exact original Write/Read/accessor bytes and matching DWARF ownership of
`Game::Instance::m_kSyncRandom`. The wire is u32 version 1, u32 seed, u32 call
count, u32 reset count, u8 synchronous and a false legacy-history byte. Every byte
matches between both canonical outputs: seed 872,261,551, call count 922,992,
reset count 0 and synchronous true. Native Write omits live `m_uiLockCount`.
This establishes equality of that complete observed serialized RNG record, not
coverage or equality of every live RNG object, lock state or asynchronous stream.

The [typed GameEffects audit](../evidence/linux/save-normalization-gameeffects.json)
now accounts for all remaining 42 bytes. Fourteen serialized object-pool IDs for
Objects::Team change from 87,481–87,494 to 87,505–87,518. Their stable Team keys are
0, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 14, 15 and 16. Twenty-eight corresponding
`contextId`/`subjectId` fields in fourteen `DEFAULT_VICTORY_REQUIREMENT` records
follow exactly that observed mapping; requirement state remains numeric 1. The
reader consumes all 28,304 RequirementSystem records and the fourteen Team suffix
records. It does not claim a global reference census across other object variants,
ModifierSystem payloads or live state.

Original Instance Read assigns each serialized object's ID before loading it,
retains temporary strong references, loads requirements/modifiers, releases those
temporary roots and trims weak object-pool entries. Original Game Read then loads
VictoryManager, whose read clears its old victory requirements, and later invokes
its own InitializeGlobalDefeats/InitializePlayer reconstruction. Team MakeObject
and requirement creation participate in that native lifecycle. These are original
Core Load operations, rather than an assumption that the omitted external host
finalizer would remove the difference. The exact allocation, reuse and destruction
sequence producing the observed +24 still needs a dedicated runtime trace. Final
object-pool allocator counters are not separately preserved by these save bytes.

[The complete bounded summary](../evidence/linux/save-normalization-summary.json)
therefore classifies all 89,198 changed bytes without masking them: Trade wire
ordering and GameEffects object/reference identities. Every exact ID, reference
and order remains recorded. A bijection or equal keyed graph payloads does not
prove future behavior, allocation order, tie breaks or global state equivalence.
No changed field was excluded as AI planning state.

The non-idempotent native Load behavior is a target to reproduce, not a defect to
patch. Same-source fresh-process native initialization already yields repeatable
serialized baselines for forthcoming scoped action comparisons. Exact original
save import must reproduce original reconstruction behavior; exact simulator
state cloning must independently retain its complete imported state. Neither
operation is established by a byte-equality expectation on native Save→Load.
