# Independent Linux CulturalIdentity / Heroes wire review

The bounded current CulturalIdentity v10 and Heroes v6 wire codecs pass this
focused review at exported committed source
`59e30ecfb4c7722a4226a3d24128541af2f1b2fb`. No implementation defect was found.
The boundary includes complete current discovery-weight and claim records.
Whole-prefix composition and subsequent Player arrays/AI framing remain separate.

The [review driver](../evidence/linux/player-cultural-identity-heroes-review.py)
and [closed evidence](../evidence/linux/player-cultural-identity-heroes-review.json)
use one immutable Git archive containing both actual crate exports and all
source dependencies:

```sh
python3 evidence/linux/player-cultural-identity-heroes-review.py \
  --output evidence/linux/player-cultural-identity-heroes-review.json
```

The report pins the archive, focused sources, generated harnesses and actual
candidate hashes. Build paths can change executable hashes; reruns record fresh
identities. No source is patched or recovered from the shared checkout. All six
actual module Cargo tests and strict
`cargo clippy --offline -p c6-core --all-targets -- -D warnings` pass there.
No original engine execution, private project writes or uploads occur.

The target remains Linux Core SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
with matching host
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The review rechecks full method bytes, exact symbol identities, concrete
bindings, virtual save/load bindings and matching DWARF:

| Codec | Complete methods | Direct/PLT bindings | Virtual bindings |
| --- | ---: | ---: | ---: |
| CulturalIdentity | 44 | 73 | 4 |
| Heroes and nested records | 98 | 183 | 10 |

Linux instructions and target identity supply acceptance; discovery reference
names and historical Windows work do not. Previous accepted profiles remain
unchanged.

CulturalIdentity writer/reader `0x2a6b4b0` / `0x2a6b5b0` establishes exactly
60 bytes: current version 10 followed by fourteen signed 32-bit words in native
writer order. The domestic-trade-route identity value and four nearby
Great-Work/Wonder identity fields are raw signed Q8 backing bits; the other
nine fields are ordinary integers. No bool, count, hash, string, padding or
implicit field occurs. Decode allocates nothing and charges zero dynamic items.
Encode checks all 60 wire bytes before a fallible exact output reserve. A
one-short item cap is inapplicable. Native historical version thresholds 2–10
are not guessed; this codec explicitly accepts only current v10. Native archive
loads/edits and initialization effects are not reproduced.

Heroes writer/reader `0x269b310` / `0x269b3a0` establishes current version 6,
a u32-counted ordered HeroClass hash/bool array, signed goody-hut discovery
chance, two independently counted i32 player-indexed vectors, a counted discovery
order and counted claims. The two player vectors preserve their positions and
lengths; their elements are integer values, not player IDs or definition hashes.
Each discovery entry has its own v6, HeroClass hash and signed weight (12 bytes).
Each claim has its own v6, HeroClass hash, claim-Era hash, signed claim turn and
last-appearance-Era hash (20 bytes). All five sequence counts are unsigned.
Every outer/nested version and bool byte is checked. There are no strings,
localized names, packed owner IDs, Q8 fields or additional tagged variants here.

HeroClass and Era values are raw definition hashes, including sentinel bits,
never SQL row ordinals. Unknown hashes, duplicates, ordering and all signed
integer bits survive transport. Native Read remaps those hashes into live
indexes and consumes unrecognized array values into temporary discard storage.
It also uses archive edit wrappers and historical defaults. The codec instead
retains the complete wire carriers, without remapping, constructor defaults,
parent linkage, signal subscription or archive dirtiness. No derived gameplay
state, RNG update, recruitment or hero choice is implemented by these codecs.

Both retained canonical traces contain actual CulturalIdentity **and** Heroes
owner hooks. This review uses those direct component cuts, matched to the same
Player parent, and checks every native CulturalIdentity end equals the following
native Heroes start. Neither record is described as context-only.

| Codec | Actual owner cuts | Total bytes | Bytes / items per record |
| --- | ---: | ---: | ---: |
| CulturalIdentity | 86 | 5,160 | 60 / 0 |
| Heroes | 86 | 46,440 | 540 / 128 |

Each stream supplies 43 of each component; counts above include both streams.
The first observed native cuts are CulturalIdentity `[5,213,599, 5,213,659)`
and Heroes `[5,213,659, 5,214,199)`. These are comparison observations, never
production parser constants. The previous closed reports and drivers remain
unchanged. Their incorrect no-owner-hook statements are qualified by the
existing additive [CulturalIdentity qualification](../evidence/linux/player-cultural-identity-owner-hook-qualification.json)
and [Heroes qualification](../evidence/linux/player-heroes-owner-hook-qualification.json).
The new review independently verifies the actual hook evidence without rewriting
that historical source/archive identity.

The independent Python grammar/item interpreter matches every Rust typed
scalar/raw-Q8 value, count, hash, boolean and complete nested record token.
All 86 direct owner records per component pass byte-exact decode/re-encode,
every proper truncation, every insufficient byte cap, exact bytes/items,
applicable one-short item caps, strict trailing rejection, uncharged one-MiB
prefix tails and two-record shared exact/one-short items. Zero-item
CulturalIdentity records work with a zero item allowance throughout.
Manually framed cases exercise distinct signed/Q8 extrema, true/false discovery
booleans, duplicate/high-bit HeroClass and Era hashes, populated discovery order
and complete claims. Mutations reject every dynamic count and nested/outer
version, plus noncanonical bools, under huge caller limits. Tiny unavailable
payloads are rejected before their declared large vectors allocate.

Native Heroes occupancy is `[0, 64, 64, 0, 0]` for discovered entries, influence
values, suzerain values, discovery order and claims. Thus the native records
exercise both integer vectors, while nonempty discovered/weight/claim records
and their nested bool/version branches have source-only coverage. The empty
Heroes wire record is 28 bytes and zero items; source tests cover it separately.
Synthetic cases and repeated records across streams do not establish native
mutation, Read or gameplay parity.

Heroes performs a complete allocation-free scan of counts, wire extents,
aggregate items and checked aggregate typed storage before materializing any
vectors with fallible reserve. Its encoder also measures complete output/items/
storage before reserving. Each dynamic entry charges one item; nested versions
and scalar fields charge only bytes. Wire-byte limits are distinct from typed
storage bounds, and no total heap-byte or allocator-overhead cap is claimed.

Heroes is an unconditional component of the parent writer. PlayerWasEverAlive,
the saved header flag and the following Unit/Feature/Continent arrays and AI
blocks are parent framing decisions. This standalone review neither reclassifies
that branch as part of Heroes nor accepts its true/false paths. Their evidence
belongs to the next composition stage, not the earlier Heroes audit.

Loading all 43 Players, full gameplay state, all gameplay RNG, actions and turns
remain unfinished. All player choices remain external policies. Unknown bytes
remain unclassified and unmasked, including the recorded 89,198-byte raw
Save→Load difference. No later AI codec is accepted by this review.
