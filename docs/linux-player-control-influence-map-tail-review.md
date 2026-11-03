# Independent Linux Control / InfluenceMap / PlayerTail wire review

This coherent stage accepts bounded Control.Manager v2 (including all five
contract types), InfluenceMap v2 and the corrected current Player v47 inline
Tail at committed source `63c16f1dd162c518435cfbe11c0bef5facea14a4`.
No implementation defect was found. These are typed saved-wire codecs, without
native Read effects, AI policy execution or complete gameplay state. No later
CityManager/Culture/other AI codec or whole-prefix composition is accepted here.

The [review driver](../evidence/linux/player-control-influence-map-tail-review.py)
and [closed report](../evidence/linux/player-control-influence-map-tail-review.json)
reproduce the stage:

```sh
python3 evidence/linux/player-control-influence-map-tail-review.py \
  --output evidence/linux/player-control-influence-map-tail-review.json
```

One immutable archive contains all three actual exports and unchanged
implementations. Every archived crate source, manifest, lockfile and relevant
audit source is checked against that Git pin before building and checked again
after validation. No export or implementation is injected after extraction.
This accepts descendants of the final exported Control validation, preserving
the preliminary altered-archive report without adopting it as exactness evidence.
The report records archive/source/harness/candidate hashes. Executable hashes
can vary with temporary build paths; each rerun records fresh identities.

Actual Cargo checks pass: two Control tests, two InfluenceMap tests, four Tail
tests and eleven inherited variant tests, plus strict
`cargo clippy --offline -p c6-core --all-targets -- -D warnings`.
The source archive is not mutated. No original engine code executes, and no
private project/artifact is written or uploaded.

The supplied Linux Core remains SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The review gates all committed method/range hashes and verifies symbol extents:

| Boundary | Complete symbol extents | Additional pinned windows | Other gates |
| --- | ---: | ---: | --- |
| Control | 23 | 0 | Five concrete Read/Write vtable pairs, factory switch, full named DWARF |
| InfluenceMap | 19 | 3 parent windows | 67 concrete bindings, two virtual slots, full DWARF |
| Tail | 25 | 0 | Complete parent writer/reader, properties host vtable, full DWARF |

InfluenceMap's 22 pins must not be called 22 complete methods: three are parent
windows, also covered by the full parent methods pinned for Tail. The Control
factory switch accepts exactly keys 0–4, and contract stream dispatch resolves
to each concrete Read/Write pair. The properties host address point and Serialize/
keyed-reader slots resolve to their exact pinned host bodies. Linux target bytes
and DWARF establish acceptance; discovery references and Windows contracts do not.

Control.Manager writer/reader `0x201cf80` / `0x201d0c0` establishes version 2,
signed player/current-priority values, a version-65,537 recycled instance list,
two bool bytes and an unsigned-counted polymorphic contract sequence. The list
retains width 16, capacity, signed slot index, Instance v2, full u32 ID, two signed
priorities, current-ID bits and the complete ordered free-index metadata. Capacity
and IDs are raw carriers, without reconstructed native allocation or live-ID
invariants. Each active instance, free index and contract slot charges one item.

Each non-null contract starts with base v3 and complete signed status/player/
priority/timing values, u32 ID/owner and raw signed Q8 extra-present value.
The derived records are complete and unframed:

| Key | Derived version | Derived fields | Total bytes including key |
| --- | ---: | --- | ---: |
| 0 | 1 | Unit/Building/District hashes, two i32 coordinates, u32 required-city carrier, bool | 69 |
| 1 | 2 | Two Technology hashes and u32 best-controller carrier | 56 |
| 2 | 1 | Two Civic hashes | 52 |
| 3 | 1 | Two i32 coordinates | 52 |
| 4 | 2 | Signed target player and DiplomaticAction hash | 52 |
| -1 | none | Null slot, no body | 4 |

Coordinate words are full i32, without i16 narrowing. Unknown keys fail before
reading an unframed body; no opaque AI skip is substituted. Raw definition hashes,
ID/owner carriers, sentinel bits, duplicate entries and ordering remain distinct
from SQL ordinals and live lookup authority. Native Manager Read resolves the
Player, initializes archives, creates contracts through a factory, updates factory
ID state and reads actual network sync configuration. It drops null live slots,
while the wire codec retains them for exact saved-byte transport. Callback/lifetime
reconstruction, definition remapping and those broader native effects are not run.

InfluenceMap writer/reader `0x2140e00` / `0x2140e50` establishes version 2 and
two independently unsigned-counted vectors of signed i16 values. They retain
all bits and ordering; neither vector is excluded or labeled planning-only.
Current component Read loads the vectors, while the enclosing Player reader
initializes InfluenceMap before reading the Player version. Live Player/archive
identity, signal subscriptions and service callbacks are not reconstructed.
The unsaved runtime relationships are not inferred to be unnecessary.

Tail takes `Context { player_version: 47, was_ever_alive }` without adding wire
bytes. Other parent versions fail. True context includes nine counted fields:
agendas, signed agenda visibilities, promotion-class hashes, district hashes,
improvement hashes, Project hash/i32 pairs, chop records, pillaging records and
signed CO2 reductions. There is no extra scalar after agendas. Chop records
retain signed raw Feature, bool, signed raw Yield and amount. Pillaging retains
signed raw Plunder, Improvement/District hashes and signed amount. Yield and
Plunder are not mislabeled definition hashes. The actual chop writer emits a
raw Feature integer although its native reader invokes hash remapping; that
asymmetry is preserved, without claiming a repaired live resave.

False context starts immediately at the common TypedVariantMap properties.
The pinned parent writer's false branch at `0x2b4f1bc` reaches `0x2b4f413`,
not the true agendas entry `0x2b4f315`; the reader's common-properties entry is
`0x2b5007f`. Both contexts then contain properties without an extra outer vector,
Improvement replacement hash/hash pairs, RandomEvent hashes and three signed
integer vectors. Encoding rejects disagreement between Context and `alive`.
Native agenda initialization, archive edits, project defaults, hash remapping
and final InitializeListeners effects are not reproduced.

Actual corpus evidence is measured independently from both original streams:

| Boundary | Records | Total bytes | Total charged items | Native boundary evidence |
| --- | ---: | ---: | ---: | --- |
| Control | 86 | 162,540 | 5,510 | Actual Control.Manager owner cuts |
| InfluenceMap | 86 | 1,204,344 | 601,656 | Same-parent Control.end to Planning.start; no InfluenceMap hook |
| Tail | 86 | 84,896 | 13,398 | BehaviorTree.Manager.end to enclosing Player.end; no standalone Tail hook |

Each stream contributes 43 records of each boundary. Every saved Player header
flag index 1 is true, including Players whose current alive flag is false.
Thus all actual tails use true context. False-to-common-properties behavior is
source-only evidence and must not be reported as an observed false Player record.
Native InfluenceMap records are 14,004 bytes / 6,996 items, with lengths
`[6996, 0]`; a nonempty religious vector is source-only. Tail native profiles
are 984/155, 992/157, 996/158 and 1000/159 bytes/items, with empty properties.

All five non-null contracts occur in actual native Control Save output. Per
stream the counts are 776 UnitRequest, seven TechResearch, one CivicResearch,
five PurchasePlot and six DiplomaticAction. Null slots and an empty Manager
remain source-only. Repeated records across streams add no native mutation
coverage. Owner offsets and contextual cuts select offline comparisons only;
production parsing has no trace offsets, signature search or opaque skips.

The independent Python grammar/item interpreter matches complete Rust typed
projections for every native record. Every truncation, exact/one-short byte and
applicable item cap, strict trailing input, one-MiB unparsed tail and two-record
shared exact/one-short item budget is checked. Source fixtures additionally
exercise all five contracts plus null, signed coordinate/Q8 extrema, duplicate
metadata, both nonempty i16 vectors, true and false Tail contexts, named Char8
properties, raw signed enums and nonempty chop/pillaging. Sixty-six huge-limit
mutations reject hostile counts, nested versions, unknown contract keys,
noncanonical bools and unsupported property modifiers. Nonempty properties are
independently walked for item charge and compared through typed-map encoding;
this does not expand support to arbitrary variant forms. Existing unsupported
types, branches and modifiers still fail. No original native mutation/Read oracle
or gameplay parity is inferred from these source fixtures.

Control and InfluenceMap preflight count arithmetic, aggregate items and required
available wire before each fallible owned-vector reserve. Control checks typed
layout; the map checks its exact two-byte element extents. Their encoders
preflight total bytes/items before output reserve. Tail-owned vectors scan the
whole grammar and checked logical storage before fallible materialization.
**The complete Tail scan is not allocation-free:** inherited TypedVariantMap
can allocate during validation and encoder measurement as well as collection.
Its Vec pushes, copies, collections and boxes may allocate infallibly. This is
an accepted transitive limitation, honestly bounded by shared remaining wire/
item credit, supported forms and depth 64. Tail-owned logical-storage accounting
does not include a promised aggregate variant heap-byte cap. No total heap-byte,
allocator-overhead or fully fallible-allocation guarantee is made.

All saved AI-owned fields remain in exact gameplay/RNG scope. No planning-only
exclusion follows from these namespaces. Loading all 43 Players, full gameplay
state, all gameplay RNG, genuine external-policy decision boundaries, actions
and turns remain unfinished. Unknown bytes remain unclassified and unmasked,
including the recorded 89,198-byte raw Save→Load difference.
