# Linux Player.Units wire envelope

`player_units::Units` is bounded current-version wire transport, including the
complete recycled Unit table and every trailing field. It does not initialize
native objects or implement gameplay state, actions or turns. Full Player and
world import remain unsupported. The recorded raw Save→Load difference of
89,198 bytes remains unmasked and unclassified.

The supplied Linux GameCore identity is SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`;
matching host identity is
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Original Linux instructions and matching DWARF establish the contracts.
Discovery references are cru121's [CC BY 4.0 research at the pinned revision](https://github.com/cru121/civ6-gamecore-reference/tree/5af389445118bed2bdebf7a8a42b219c8fba5175);
see [attribution](reference.md). No cross-platform contract equality is assumed.

Current Units version is 22 (`0x2f69c10`). The writer is `0x2ba1a80`,
986 bytes; reader is `0x2ba1e60`, 3,010 bytes. The table writer is
`0x2ba8f30`; reader is `0x2ba9220`. The table writes version 65,537,
16 index bits, capacity, live count, and ordered `(i32 slot, Unit v28)`
entries, then allocator current-ID bits, free count, head, tail and exactly
free-count allocator words without a second count. Slots must be unique,
nonnegative and below capacity. Allocator words are retained without inferred
identity or ownership semantics. Full Unit IDs and owners are independent of
slots and the enclosing Player ID, which this envelope does not contain.

After the table, the exact sequence is:

1. Unit-cost, Unit-production Q8, extra-copy and promotion-class wall-attack
   hash arrays, followed by the Unit build-disabled hash/bool array.
2. No-settling, no-foreign-heal and no-neutral-heal booleans; ordered district
   founding map entries, each with an ordered nested u32/u32 map.
3. Free-pass and embarkation-pass Unit definition hashes; extra build charges,
   per-Unit extra build-charge hash array, extra spread charges, embarked
   movement modifier and formation-upgrade-on-conquest count.
4. Counted random promotion selections, proposed promotions and base
   promotions. Each record has its own version 22. Their Unit/promotion types
   are hashes; proposed Unit IDs retain all 32 bits; selection counts are i32.
5. Unlimited-promotion-choice Unit hashes; four civic hashes, four technology
   hashes; peak Unit count and six signed strength/wall-attack/album modifiers.
6. Banned/favored Yield raw i32 ordinals, favored-production and upgrade-cost
   raw Q8 values, Unit-resource-cost hash array and Resource-fuel-demand hash
   array.

Unit and promotion enum overloads (`0x2d4ec50`, `0x2d4ecc0`) explicitly
convert definitions to hashes, including generic sequence element dispatch
(`0x2836620`, `0x282b890`). Yield's scalar overload (`0x2d4fb70`)
retains a raw ordinal. These identities are not SQL row positions. Ordered
arrays/maps preserve duplicate keys and unknown hashes without normalization.
Unit transport and its existing nested-version, variant and nine-operation
handler refusal gates remain in effect. Unit wire omits domain; native Read
also performs live definition/domain/archive/dirty/facade fixups which are
outside this codec.

`decode`, `decode_prefix` and `decode_prefix_with_items` distinguish strict
and prefix boundaries. `encode_with_items` reports the same aggregate item
count as decoding. Each counted sequence element, live slot, free allocator
word and nested Unit child consumes the shared item allowance. Prefix bytes
consume the shared byte allowance; trailing enclosing data does not. Each
envelope-owned allocation uses checked addressable layout and fallible reserve
after its count, byte-minimum and item checks. This guarantee applies to the
new Units envelope implementation. Inherited nested Unit transport retains
`Vec::with_capacity` in `unit/state.rs` sequence decoding and infallible Vec
extension in its writer after existing count/wire checks; the complete nested
codec therefore does not guarantee fallible allocation failure handling.
The shared limits bound aggregate serialized bytes/items across nested Units,
rather than total heap bytes. Native runtime allocation/ID reconstruction is
not reproduced.

The read-only audit `tools/linux/player_units_audit.py` builds from one
immutable actual source commit in an isolated archive, verifies ELF contracts
and compares native owner cuts from both canonical streams. Owner cuts are
validation inputs only; the codec uses source-derived counts and schemas.
First observed Units bytes begin at 5,008,068; this is not a parser constant.
The next Player schema is Eras, established by the enclosing writer's next
serializer call at `0x2b4f10c` after Units at `0x2b4f0f9`.

The initial frozen-source audit passed for source commit `8119a34`
(report history `c271899`): 209 Linux method
extents, 1,765 concrete PLT bindings and matching Units/promotion DWARF. Each
canonical stream supplied 43 complete Units envelopes (6,422,174 bytes),
including all 1,262 nested Units (6,137,501 bytes), with exact decode/encode
bytes and independently attributed nested boundaries. Three source tests cover
nonempty trailing carriers, allocator metadata, duplicate hashes, every
synthetic truncation, versions, booleans and limits. Native checks cover exact
and one-short byte/item limits, trailing/prefix behavior, duplicate/invalid
slots, altered full Unit ID/owner preservation, the first 32 and final 256
prefixes of each envelope and truncation before every nested Unit end. They do
not claim every byte prefix of every large native envelope was tested.
The [public report](../evidence/linux/player-units.json) records source,
archive, candidate, harness and native record identities; it contains no
proprietary payload or raw disassembly.

A documentation review clarified the allocation-policy description after that
initial closed audit run. Commit `1184f1c` preserved its actual source, archive,
candidate, harness and audit-tool identities and recorded the metadata-only
correction explicitly, without rerunning the corpus for the wording change.

The free-pass sequence alone has a signed i32 count (writer `0x2ba1bae`,
reader `0x2ba2136` with sign extension at `0x2ba214a`). Its envelope adapter
rejects negative/high-bit carriers before allocation even under maximal
Limits, and encoding checks the length with `i32::try_from`. Other sequence
count grammars remain unchanged. `--source-commit COMMIT` selects the actual
immutable source checkpoint for a new isolated audit run.

The signed-count follow-up was audited from actual source/driver commit
`a4fb0ab`. All 86 original envelopes and 2,524 nested Units across both streams
again matched byte-for-byte and at independently attributed boundaries;
observations and ordered payload digests match the initial run. Four source
tests now include the maximal-Limits negative-count refusal and checked i32
encoder boundary, in addition to the existing envelope checks. The current
public report records the fresh source/archive/candidate/tool identities and
the inherited allocation limitations stated above. No original code was
executed and no private evidence or project was written.
