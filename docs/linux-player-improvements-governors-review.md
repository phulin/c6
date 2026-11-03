# Independent Linux Improvements / Governors wire review

The corrected current Improvements v1 and Governors v6 wire codecs pass this
focused review at composition source
`356109b8b8e4c79a9e76a618cf108fe082dc7568`. No implementation defect was found.
The accepted boundary is saved wire transport, including Governor v16 and
Grievance v1 records. Whole-prefix composition/producer acceptance remains
separate; this review stops before CulturalIdentity.

The [review driver](../evidence/linux/player-improvements-governors-review.py)
and [closed report](../evidence/linux/player-improvements-governors-review.json)
reproduce the checks from one immutable archive:

```sh
python3 evidence/linux/player-improvements-governors-review.py \
  --output evidence/linux/player-improvements-governors-review.json
```

The report records the archive, focused sources, generated harnesses and actual
candidate hashes. Build paths can affect executable identities; reruns record
fresh hashes. Source files are never recovered from the shared checkout. The
archive runs the five actual Cargo source tests and
`cargo clippy --offline -p c6-core --all-targets -- -D warnings`; all pass.
Original engine code is not executed, and no private project/artifact is written.

The Linux target remains Core SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with matching host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The review rechecks all complete target method extents, symbol identities,
concrete bindings, virtual bindings, current versions and matching DWARF:

| Codec | Methods | Direct/PLT bindings | Virtual bindings |
| --- | ---: | ---: | ---: |
| Improvements | 26 | 35 | 4 |
| Governors and transitive records | 53 | 218 | 0 |

Discovery reference names and historical Windows contracts do not supply
acceptance. Existing accepted profiles are preserved.

Improvements writer/reader `0x2b1cd70` / `0x2b1cdc0` establishes version 1,
unsigned plot count, ordered signed plot indices and a final independent u32
array-size word. The complete extent is `12 + 4 * count`. Plot indices are not
hashes, and the final size need not equal count. It is never synthesized from
the vector. No bool, string, Q8 or nested version occurs. Native Read clears and
reloads its existing vector and assigns the independent size through native
archive machinery; the codec does not reproduce those live/sync effects. It
also establishes no Map Plot or City improvement state coverage.

Governors writer/reader `0x2b09260` / `0x2b092e0` establishes version 6,
signed points, a recycled Governor list, signed neutralization count, title
bool, favored Governor definition hash and signed diplomatic favor. Starting
Governor count, maximum titles and the parent pointer are not invented as
wire fields. The recycled list is version 65,537, width 16, capacity, unsigned
active count and ordered signed slot/Governor entries, followed by current-ID
bits, unsigned free count, head, tail and exactly free-count u32 words. Capacity
is retained metadata, not a request to allocate capacity-sized native storage.
No missing slots or allocator words are filled with zeroes.

Governor writer/reader `0x2a36430` / `0x2a36660` establishes version 16.
The complete nested grammar retains ID, definition hash, counted encoding-0
name bytes, assigned CityID, PlayerTypes owner, six site/pressure integers,
established bool, seven neutralization/identity/alliance/modifier integers,
assign-to-major bool, three resource/faith/heal integers, established-city map,
promotion hash/bool array, city-state bool and counted Grievances. Grievances
carry their own version 1, signed score and signed turns. Names preserve raw
bytes, including embedded zeroes, without Unicode/localization normalization.
CityID is exactly signed i16 owner, signed i16 component type and unsigned u32
ID. The codec does not mask those halfwords, combine them into a guessed ID,
resolve them or infer ownership authority. Governor and promotion identities
remain raw definition hashes; the owner remains a raw PlayerTypes ordinal.
Signed modifiers are retained as integers, without guessed Q8 conversion.

Established-city entries retain the CityID and signed value in wire order,
including duplicates. Their count is emitted unsigned but consumed signed by
native Read; high-bit/negative carriers are explicitly refused before typed
allocation. Encoding enforces the same signed representability bound. Other
sequence counts are unsigned. All current outer/nested versions and every bool
are checked. Native definition overlays, map insertion, promotion storage edits,
callbacks, native allocation and RNG/gameplay effects are outside transport.

Governor slot policy requires a separate distinction. This codec intentionally
retains raw signed slot words, ordering, duplicates and high bits; it does not
claim valid live-container reconstruction. Native Write emits ascending unique
nonnegative indices. Native Read passes slot bits to AddAt, whose low-16-bit
index handling can reserve beyond the saved capacity and alias or overwrite an
existing object. Those normalization/allocation effects are not reproduced.
The synthetic negative/duplicate slot cases prove preservation only, not that
native Write produces them or native Read byte-roundtrips them. The codec's
active-count <= capacity check is an explicit saved-container consistency policy,
not native Read acceptance equivalence. No unsupported native slot rejection
is invented for this transport.

Both streams contribute 43 Improvements and 43 Governors envelopes each:

| Codec | Total bytes | Total charged items |
| --- | ---: | ---: |
| Improvements | 21,624 | 5,148 |
| Governors | 90,826 | 17,678 |

There are **no direct outer Improvements or Governors owner hooks**. Quests-end
and enclosing Player-end provide context. The source-closed WMD count grammar
locates Improvements, and its `12 + 4 * count` extent, including the independent final word,
locates Governors. The candidates receive these entire remaining suffixes. Improvements
consumption is independently compared with `12 + 4 * count`; Governors is parsed
by an independent Python grammar. Every Governors end also matches the actual
next Player.CulturalIdentity hook entry. This is not a fabricated outer owner
cut and does not place offsets or traces in production parsing.

Native Governor.Instance hooks do exist. The review compares every nested
start/end with those independently attributed calls: 108 instances per stream,
216 total. The corpus has 17 nonempty and 26 empty Governors envelopes per
stream, hence **34 nonempty and 52 empty** envelopes across both streams.
The 53-byte empty record charges zero items. An 86-record count must not be
reported as 86 nonempty envelopes. Repeated records do not establish new native
mutation or branch coverage.

The independent Python interpreter matches every Rust typed scalar, count,
boolean, hash, name byte, CityID halfword, free word and Grievance token, as well
as aggregate item charges. Every native record passes exact decode/re-encode,
every truncated prefix, exact/one-short bytes and applicable item caps, strict
trailing rejection, an uncharged one-MiB tail and two-record shared exact/short
items. Zero-item records are accepted without inventing a one-short item case.
Manual synthetic records exercise duplicate/extreme plots and an unequal final
size, arbitrary name bytes, signed extrema, duplicate maps/promotions/free words,
all nested records and preserved negative/duplicate slot carriers. Hostile
mutations cover every count, bool and outer/nested version under huge limits;
invalid ID width is rejected. A huge capacity with only the supplied live entries
still roundtrips, demonstrating that capacity is not used as an allocation count.
Synthetic cases are source checks, not a native mutation oracle.

Improvements preflights the complete counted payload including its independent
tail word and addressable typed storage before fallible reserve. Governors
performs a complete scan without dynamic allocation, checking every nested
extent, aggregate items and cumulative logical typed storage <= isize::MAX.
Only then does a second pass reserve the typed vectors fallibly. Its encoder
likewise measures complete wire/items/storage before output allocation.
Each live slot, string byte, map/promotion/Grievance entry and free word charges
one item. Scalar and version words charge bytes only. Typed storage is not
equated with the wire-byte cap, and no total heap-byte/allocator-overhead cap
or native allocation equivalence is claimed.

All 43-Player loading, full gameplay state, all gameplay RNG, action execution
and turns remain unfinished. Governor/improvement choices and all other player
decisions remain external policies. The separate later AI::Governors state is
not excluded by its namespace. Unknown bytes remain unclassified and unmasked,
including the recorded 89,198-byte raw Save→Load difference.
