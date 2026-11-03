# Independent Linux Stats / TrackedValue / Techs review

The bounded current wire codecs at exported source commit
`f09cba77c388402e7d6de64f5841ddb336a3d410` pass this focused review.
No implementation defect was found. This accepts their individual transport
boundaries; the expanded byte-zero Core/Player prefix through Techs must have its
own frozen composition and shared-budget review before acceptance.

The reproducible review is [the independent script](../evidence/linux/player-stats-techs-review.py),
with [closed results](../evidence/linux/player-stats-techs-review.json). Run:

```sh
python3 evidence/linux/player-stats-techs-review.py \
  --output evidence/linux/player-stats-techs-review.json
```

The script builds one `git archive` of that commit, including all three public
exports and their committed dependencies. It never repairs the archive with
mutable checkout files. It hashes the archive, candidate, harness and source
files. Executable hashes identify each specific build, including its temporary
build paths; rebuilding records a fresh identity. The same single candidate
replays all components. It reads the hash-gated original Linux ELF and existing
canonical records, without executing original game code or writing private
artifacts. No Windows acceptance is used.

The target Core remains
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with matching host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The review rechecks concrete full method fingerprints and symbol identities
(61 Stats, 15 TrackedValue/localization and 27 Techs entries), current version
words, matching DWARF and neighboring Player writer calls. Original writer and
reader instructions were reviewed with the schema and adapters. Discovery
reference names are leads only; pinned target bytes establish these contracts.

Stats writer `0x2b7c710` and reader `0x2b7cb00` establish version 27. Its
22 initial signed words precede the unframed nested TrackedValue. Favor and raw
Q8 values follow in writer order, then Unit/Project hash-value arrays, the signed
plot vector, Yield hash-Q8 pairs, seven count-prefixed arrays of exactly 64 signed
words, and 16 disaster counters. Native member order differs from wire order;
no field sorting or ordinal conversion is allowed. The seven literal 64 writer
arguments were checked directly. Raw u32 hashes, signed extrema and duplicates
remain observable through transport.

TrackedValue save/load `0x2d50a60` / `0x2d50c00` establish version 1, signed raw
Q8, include flag, optional minimum and maximum, and a signed step count. Each
step writes signed priority, unsigned ID, localization and a recursive versioned
TrackedValue. The step loop does not depend on the include flag. There is no
nested length envelope. Its codec neither clamps values nor applies native load
initialization/overlay semantics. Strict canonical bool handling and current-only
versions are explicit typed support boundaries.

Techs writer/reader `0x2b89f90` / `0x2b8a090` establish version 6. Queue
`0x2b8f680` / `0x2b8f610` writes an unsigned count and iterates the whole queue;
the C++ template parameter 4 is no wire cap. The TechTypes operators
`0x2d4eef0` / `0x2d4ef20` use definition hashes. Overflow and science change
are signed raw Q8, modified boost is signed i32, two ordered hash/bool arrays
precede ordered hash/Q8 research progress, and three signed words finish the
record. Native Reset, historical TreeManagement migration and definition overlay
are outside this transport. Decoding does not invoke the runtime tech provider.

Both canonical streams supply 43 Stats records, 43 enclosed TrackedValue prefixes
and 43 Techs records. Across both streams, the review compares 282,098 Stats
bytes, 1,290 tracked bytes already included in Stats, and 123,324 Techs bytes.
Each record passes byte-exact decode/re-encode and item-charge comparison,
every-prefix truncation rejection, strict trailing rejection, an uncharged
prefix tail with exact bytes/items, unsupported-version rejection, and exact
versus one-short bytes/items on both decode and encode. Owner trace offsets
are offline fixture selectors; production codecs receive no trace metadata.
This is saved-record comparison, not a native mutation oracle.

Stats records have 116 Unit pairs, 34 Project pairs, 6 Yield pairs, 0–8 lost
wonder plots and 605–613 items. Techs records have one queue hash and 77 pairs
in each of the three hash arrays, for 232 items. All recorded tracked prefixes
are 15-byte empty histories, include=true, without optional bounds, costing one
item. Therefore nonempty/duplicate steps, include=false, optional bounds,
recursive localization and depth limits have source-test coverage only. The
13 relevant source tests also exercise signed extremes, duplicate/unknown
hashes, hostile counts, malformed bools and fixed-array counts, and storage
larger than serialized bytes. These tests passed from the same archive.

Resource policy remains part of the accepted boundary. Stats charges all 448
fixed-array elements and its dynamic entries. TrackedValue charges every node,
including the root, every step, and localization's dynamic elements/text bytes.
Nested codecs report consumed bytes/items into the caller's remaining budget;
trailing caller bytes are not charged. Tracked and localization recursion each
have an explicit depth-64 codec policy, with tracked root depth zero; this does
not establish a native maximum. Decoding rejects excessive depth before building
another node, while callers remain responsible for deeper manually built trees.
Vectors check multiplication/addressability and reserve fallibly. The limits
bound wire bytes and aggregate items, without promising an aggregate typed-heap
byte cap. Localization derives checked logical storage from remaining items.
Techs prechecks a bool array's complete wire input, item count and typed layout
before allowing the helper sufficient separate typed storage; the helper's
original per-vector byte/storage coupling is deliberately relaxed.

Full 43-Player loading, complete gameplay state, all gameplay RNG, actions and
turns remain unfinished. This review adds no native Read remapping or gameplay
transition. Unknown save bytes remain unclassified and unmasked, including the
recorded 89,198-byte raw Save→Load difference. The next component is Treasury;
the Resources end remains an internal proof point in the expanded prefix.
