# Independent Linux post-Trade component review

The bounded current GreatPeoplePoints v13, Influence v22, Quests v1 and WMDs v1
wire codecs pass this focused review at committed source
`8a99f1fba96a5f189545751c323a4a6ce2f35e11`. No implementation defect was found.
The first three implementation files are unchanged from composition `986770d`.
This review accepts individual wire boundaries; whole-prefix composition and
producer metadata have separate review and acceptance.

The [reproducible script](../evidence/linux/player-gpp-influence-quests-wmds-review.py)
and [closed evidence](../evidence/linux/player-gpp-influence-quests-wmds-review.json)
use one immutable archive with the four public exports and dependencies:

```sh
python3 evidence/linux/player-gpp-influence-quests-wmds-review.py \
  --output evidence/linux/player-gpp-influence-quests-wmds-review.json
```

The report records the full source commit, archive hash, focused source hashes,
actual harnesses and candidate executables. Executable hashes can vary with
build paths; each run records fresh identities. No shared-tree files repair
the archive. No original engine runs, raw disassembly, private writes or uploads
are used. The target remains Linux Core SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and matching host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.

The review rechecks every complete method extent in each committed audit,
symbol identities, concrete direct/PLT bindings, virtual bindings, current
version words and matching DWARF. These are Linux target gates; discovery
reference names or historical Windows contracts do not supply acceptance.

| Codec | Methods | Direct/PLT bindings | Virtual bindings |
| --- | ---: | ---: | ---: |
| GreatPeoplePoints | 105 | 344 | 7 |
| Influence | 76 | 294 | 14 |
| Quests | 11 | 29 | 0 |
| WMDs | 13 | 27 | 0 |

GreatPeoplePoints writer/reader `0x2b10cb0` / `0x2b10e80` establishes version
13, saved PlayerTypes i32, three Great Person class hash/Q8 tables, free points
and refund percentage i32, two bool bytes, and three more signed-value tables.
Those tables respectively use Yield integer discounts, Great Person class
integer percentages and Yield raw Q8 modifiers. Three unsigned-count vectors
follow: yield changes, earned boosts and class hashes. Each yield change has
its own version 13, Yield/Class/Resource definition hashes and signed amount
(20 bytes); each boost has version 13, Class hash and two bools (10 bytes).
The final class entries are hashes, not ordinals. There are no fixed arrays,
strings or omitted transitive records in this grammar.

Influence writer/reader `0x2b1ed50` / `0x2b1eff0` establishes version 22.
Its saved owner and suzerain are raw signed PlayerTypes ordinals. Points and
three Yield tables retain raw signed Q8 bits; token and modifier values remain
signed integers. The source order of scalars, eight i32 vectors, one bool vector
and three Yield hash tables is retained. Every vector has its own unsigned count;
64 is an observed vector length, without a fixed-array rule. The unsaved parent,
initialization flag and giving-tokens-considered member are not invented as wire
fields. This establishes no separate DiplomaticRelations grammar or authority.

Quests writer/reader `0x2b5cb40` / `0x2b5cbe0` establishes exactly eight bytes:
version 1 and saved PlayerTypes i32. The null-parent writer's -1 value and all
other ordinal bits survive unchanged. There are no collections, booleans,
strings, definition hashes or QuestData records here. Zero collection items is
valid, and a one-short item cap is inapplicable. Native Read consumes its version
without branching; the typed codec explicitly accepts only current version 1.
This narrow parent carrier does not establish absence of gameplay quests,
NPC quest state, quest generation or their RNG elsewhere.

WMDs writer/reader `0x2bc5fb0` / `0x2bc6000` establishes version 1, an unsigned
count and ordered `(u32 WMD definition hash, i32 weapon count)` pairs. Counts
are integers, not Q8. No saved parent ordinal, array-size member, bool, string
or nested version occurs. The complete size is `8 + 8 * count`; high-bit count
carriers remain unsigned and must pass caps and complete-input preflight before
allocation. The lint follow-up at `8a99f1f` changes constant-width iteration to
`as_chunks::<8>()`; the complete-extent check preserves its framing.

Across all four codecs, definition hashes stay distinct from SQL ordinals and
live definition indexes. Ordered pairs, duplicates and unknown hashes survive;
no lookup default, sort, clamp or native overlay is substituted. PlayerTypes
values remain raw i32 ordinals, without establishing live player lookup or
relationship authority. No codec serializes a packed archive ComponentID.
Native Initialize operations for the first three records assign parent/archive
ownership; GPP and Influence also reset their native data. Their native readers
use definition remapping and archive edits where applicable. Quests initializes
its parent/archive but does not call Reset. WMDs Read edits an existing synced
array and remaps hashes, without Initialize or Reset. Unknown WMD hashes still
consume counts, while recognized duplicates overwrite native destinations.
The wire codec preserves all entries instead. None of these native effects,
callbacks or transitive gameplay/RNG behavior is reproduced by transport.

All four components compare 86 records across the two original canonical streams:

| Codec | Total bytes | Bytes / items per record | Boundary evidence |
| --- | ---: | ---: | --- |
| GreatPeoplePoints | 37,668 | 438 / 48 | Actual component owner cuts |
| Influence | 182,750 | 2,125 / 530 | Actual component owner cuts |
| Quests | 688 | 8 / 0 | Actual component owner cuts |
| WMDs | 2,064 | 24 / 2 | Contextual suffix and independent count framing |

There are **no WMD owner hooks** in these traces. Each Quests-end and its
enclosing Player-end delimit a contextual suffix. The WMD candidate receives
that entire suffix; an independent audit-only unsigned-count calculation
checks its consumed extent and exact bytes. The resulting boundary is not an
independently traced WMD end. This distinction is retained in the report and
does not turn context cuts into production offsets. All production parsers
follow typed counts and fields without owner traces, searching or opaque skips.

An independent Python grammar and item interpreter matches every Rust typed
scalar, count, hash, integer/Q8 value and boolean token, including current nested
GPP versions. Every original record passes exact decode/re-encode bytes/items,
every truncated prefix, exact and one-short byte caps, exact and one-short
nonzero item caps, strict trailing rejection, an uncharged one-MiB prefix tail,
and shared exact/one-short items across two concatenated records. Quests passes
with zero items throughout. The eight committed focused source tests also pass.
Additional manually framed cases exercise distinct signed scalars, duplicate
high-bit hashes, signed extrema and nonempty GPP nested vectors; hostile mutations
cover every dynamic count, bool byte and nested/outer version under huge limits.
They fail before their declared unavailable payloads can allocate large vectors.

Native GPP records contain six tables with 9/9/9/6/9/6 entries and three empty
vectors. Nonempty yield-change, earned-boost and no-points vectors therefore have
source coverage only. Native Influence records contain an empty per-player levy
discount vector; its nonempty case is source-only. Duplicate/unknown hashes and
extreme synthetic values likewise do not constitute native mutation comparisons.
Repeated records/shapes across streams do not establish new native branch coverage.
Live native serialization, Read effects and gameplay parity remain gaps.

GPP and Influence count every collection element or table pair once; nested
version/scalar fields add only bytes. Each count preflights arithmetic, aggregate
items, minimum wire extent and available input before fallible typed reserve.
GPP's largest dynamic entry is 16 typed bytes; Influence's is eight. Those
logical storage bounds follow charged items, separately from wire bytes.
Their encoders measure the complete output and item charge before reserving.
Quests decodes without allocation and checks eight output bytes before reserve.
WMDs checks the full counted payload and addressable typed layout before reserve.
No total heap-byte cap, allocator-overhead bound or runtime allocation parity is
claimed.

Complete 43-Player loading, full gameplay state, all gameplay RNG, actions and
turns remain unfinished. Great Person recruitment, envoy assignments, quests,
weapon deployment and all other player decisions remain external policies.
Unknown save bytes remain unclassified and unmasked, including the recorded
89,198-byte raw Save→Load difference. WMDs is followed by Improvements; this
review does not accept that later component or the expanded full prefix.
