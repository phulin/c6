# Independent Linux Treasury / Units / Unit-prefix review

The repaired bounded codecs at source commit
`a4fb0ab8d9431f3e2d03f02f0b62f17df844535b` pass this focused review:
Treasury v10, the complete Player.Units v22 recycled roster envelope, and
the new Unit v28 prefix/item APIs. The expanded byte-zero
Core prefix has its own composition review. These codecs transport current wire
records; they do not load live native objects or implement gameplay state.

Reproduce the single-archive validation with
[evidence/linux/player-treasury-units-review.py](../evidence/linux/player-treasury-units-review.py):

```sh
python3 evidence/linux/player-treasury-units-review.py \
  --output evidence/linux/player-treasury-units-review.json
```

The [closed report](../evidence/linux/player-treasury-units-review.json) records
the exact committed source, archive, source hashes, generated harnesses and
executables. One immutable archive supplies all dependencies and public exports.
The harnesses reuse committed native-record checks and add independent item
algebra, raw malformed-slot decoder cases and allocator metadata checks.
Executable identities are specific to each build; reruns record new identities.
No mutable source recovery, original game execution or private writes occur.

The ELF target remains Core
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The review rechecks full target method fingerprints, symbol identities,
concrete PLT bindings, current-version words and matching DWARF: 51 methods /
252 bindings for Treasury and 209 methods / 1,765 bindings for Units. Source
reference names remain discovery leads. Linux target instructions and original
saved records establish this acceptance; historical Windows work contributes
no acceptance.

Treasury writer `0x2b99260`, reader `0x2b99350` and version word `0x2f6937c`
establish current version 10. The wire has ten signed words in writer order,
including raw Q8 values, followed by an Improvement hash/Q8 array and ordered
unsigned u32/u32 diplomatic-deal pairs. Hashes remain raw u32 identities;
no ordinal remapping or native archive edits are applied. Historical reader
branches and live definition overlays are excluded. The review checks all
86 original Treasury records, totaling 50,552 bytes across both streams.
Each record passes byte-exact decode/re-encode, every truncated prefix rejection,
and exact versus one-short byte/item limits. Source tests additionally cover
signed/unsigned extremes, duplicate hashes, versions and hostile counts.

Unit prefix wrappers share the existing strict v28 parser and field-range
construction. Strict decode still rejects a full input exceeding its byte cap
and requires exhaustion. Prefix decode clips the available input to the byte
cap, reads one source-derived record and returns consumed bytes/items without
charging the caller's unparsed tail. Clipped truncation, including a nested
variant truncation, reports ResourceLimit; semantic version/variant/handler
errors remain intact. Encoding reports the existing writer's exact dynamic
item charge. All nested versions, nine-operation handler refusal and supported
variant restrictions remain in force. This does not add native domain fields,
definition fixups, live pointers, dirty history or gameplay effects.

The review checks all 2,524 original Unit cuts as prefixes of the remaining
Core stream, without supplying traced ends to prefix parsing. Each passes
strict versus prefix equality, exact bytes/items and one-short rejection,
byte-exact encoding, strict trailing rejection and shared exact/one-short item
accounting over concatenated records. Every truncated and clipped prefix of
the first 4,870-byte Unit in each stream is rejected. The inherited strict
checker also passes 2,524 Unit, 2,524 queue and 2,500 Formation roundtrips and
17,668 independently traced nested Unit field boundaries. These figures describe
wire validation; the retained timing fields make no gameplay benchmark claim.
Repeated component bytes across the two streams count as records, without
establishing new native mutation or branch coverage.

The Units writer `0x2ba1a80`, reader `0x2ba1e60` and version word `0x2f69c10`
establish current version 22. Recycled table writer `0x2ba8f30` / reader
`0x2ba9220` writes version 65,537, literal index width 16, capacity and live
count, ordered signed slot indices with Unit v28 records, current-ID bits,
free count, free head/tail and exactly free-count free-entry words without a
second count. The codec retains every word and its ordering. It validates
unique nonnegative slots below capacity; it neither reconstructs native
allocation nor synthesizes unused slot or free-chain values. Unit full IDs
and owners remain independent fields.

The review found a signed-count edge in the first Unit free-pass sequence.
Writer `0x2ba1bae` uses an i32 count; reader `0x2ba2136` reads i32, sign-extends
it for resize and uses a signed loop. The initial generic u32-count adapter
rejected high-bit values through the audited 64 MiB / 4-million-item limits,
without an independent signed-count guard under arbitrary larger caller limits.
`a4fb0ab` resolves this with a negative-count refusal before allocation or item
charge and checked i32 encoding length. The public Vec field and all positive
wire bytes remain unchanged. Huge-Limits tests reject both `0x80000000` and
`0xffffffff`, test encoder count boundaries without a huge allocation, and
verify full-envelope dispatch. The explicit replacement of the record macro
retains all 42 field read/write positions. Other sequence counts are unchanged.

The complete trailing sequence was checked against native dispatch and DWARF:
Unit cost/production/extra-copy and promotion-class arrays; build-disabled
hash/bools; three booleans; nested district-founding maps; two Unit hash
sequences; build/spread/movement/formation modifiers; the three versioned
promotion record vectors; unlimited-promotion-choice Unit hashes; four civic
and four technology hashes; peak Unit count and six signed modifiers; two
raw Yield ordinals; two Q8 modifiers; and Unit/Resource cost-demand hash arrays.
Unknown and duplicate hashes/map keys retain their wire ordering. Unit and
promotion enum overloads serialize definition hashes, while scalar Yield
serialization preserves an ordinal. Neither SQL row positions nor generic
AI/planning exclusions are inferred.

Both canonical streams supply 43 complete Units envelopes each: 12,844,348
bytes in total, including 2,524 Unit records totaling 12,275,002 bytes. Nested
boundaries and slot positions match independently traced native calls. Exact
allocator metadata and free-word sequences match the typed fields without
normalization. Independent item algebra sums every envelope element, live
slot, free word, nested map entry and child Unit charge. Per stream the envelope
charges 817,224 items, including 780,570 nested Unit items.

Every original envelope passes exact and one-short byte/item limits, strict
trailing rejection, exact prefix-tail handling, first 32/final 256 truncated
prefixes and truncation before every nested Unit end. This does not claim
all byte prefixes of each large native envelope were tested. The 25 focused source tests
(3 Treasury, 4 envelope and 18 Unit/component/variant) pass, including all
synthetic prefixes and nonempty trailing fields. Additional review
mutations reject outer/table versions, index width, hostile live counts,
duplicate slots, negative slots and slots equal to capacity in decoding.
Existing mutations retain altered full Unit IDs/owners and reject child Unit
version changes. Native absent branches remain source-only evidence.

Envelope-owned allocation checks occur before reserve: remaining aggregate
items, minimum wire bytes, available input and addressable typed layout.
Their reserves and output growth are fallible. Nested accepted Unit vectors
retain their prior `Vec::with_capacity` and output extension policy after
item/minimum-wire-byte checks. The complete nested transport therefore makes
no promise that all allocation failures are returned as errors, and no
aggregate heap-byte cap is claimed. This documentation finding is corrected
in `1184f1c`; it changes no tested source or candidate identity.

The first observed Treasury is 780 bytes / 91 items. The following Units
envelope is 183,883 bytes / 23,308 items, with capacity 48, 36 live slots and
12 free-entry words. These are validation observations, never parser constants.
The next component is Eras. Complete 43-Player loading, full gameplay state,
all gameplay RNG, actions and turns remain unfinished. Unknown save bytes
remain unclassified and unmasked, including the recorded 89,198-byte raw
Save→Load difference.
