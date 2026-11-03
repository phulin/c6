# Linux post-Map Player prefix

[`core_player::Prefix`](../crates/c6-core/src/core_player.rs) reads the current Linux raw Core wire stream beginning immediately after Map. It decodes Game.Religion7 and every ReligionData7, Player.Manager1's framing, the first Player.Instance47 header, the complete first Formation collection, Goody_Hut1, Bonuses4, Congress12 and **the complete Player.Cities15 collection and its trailing fields**. It stops before unframed Player.Diplomacy. It does not finish the first Player, reach the remaining Players, establish a full City/Formation/Unit roster, load a live game or execute actions/turns.

`Prefix::decode_prefix_with_items(bytes, unit::Limits)` returns `(typed_prefix, consumed_bytes, dynamic_items)`; `decode_prefix` omits the item charge. The consumed position identifies a prefix boundary, not exhaustion of a complete Player record. `Boundary::PlayerDiplomacy` and `unfinished_player_entries` explicitly retain that distinction: for a nonempty Manager the unfinished count includes the partially read first entry. Empty Managers return `AfterEmptyPlayerManager` without consuming later Game components. Trailing input is untouched. `encode_prefix_with_items` returns the encoded supported prefix plus its exact item charge; `encode_prefix` omits that charge. Encoding refuses inconsistent boundary/count/first-entry combinations and writes no unsupported tail. There is intentionally no strict full-Player decoder or opaque unframed skip.

Manager's requested owner and the record's embedded owner remain separate fields. The original Manager writer obtains the ID from the same Player it then serializes; this supported writer-wire grammar requires equality and rejects a mismatch. It does not infer a roster-index/player-ID mapping or reproduce Manager.Read initialization and fixups. The header retains its fifteen ordered i32/enum scalars and six bools without speculative scalar names; hashes remain full u32 words, and religion names are counted byte strings without UTF-8 normalization. Formation and City owners are preserved as written without rewriting them to the Manager ID.

## Exact source contracts

The target is Linux Core SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, with matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. Existing [Core layout](linux-core-stream-layout.md), [Player components](linux-player-components.md) and [complete City wire evidence](linux-city-wire.md) supply the accepted preceding/current child grammars. Original execution remains identity/configuration gated; this implementation does not execute original code. Reference layout leads use cru121's pinned revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, CC BY 4.0, with [attribution and limitations](reference.md).

Relevant original writer/reader addresses are Religion `0x2a157c0`/`0x2a15820`, ReligionData writer `0x2a15560`, Player.Manager `0x2b51060`/`0x2b510f0`, Player.Instance `0x2b4ee00`/`0x2b4f490`, and Player.Cities writer `0x2a579d0` (231 bytes). These are source evidence, never parser offsets. The existing Formation strict codec validates the exactly sized current record: fixed 36-byte prefix, count plus 20-byte PathNode records, cached coordinate and counted u32 member IDs. Those counted lengths determine its slice dynamically.

City recycled-container writer `0x2a5b840` (374 bytes, existing checked SHA-256 `2819e602eddaaf3faebfcdc440a62e75550c30d02047c90cc976adb0b81a41bc`) writes version65537, index-bits16, capacity, live count, then each **explicit signed slot plus complete City51**. The trailing metadata is current generated-ID bits, free count, free head, free tail, then exactly free-count u32 links **without another count prefix**. The same framing is used for Formations. Entry order and allocator words are retained; duplicate/out-of-range live slots reject, and generated IDs are never used to synthesize slots. Zero live entries still consume all allocator metadata and manager tail fields.

After this container, Player.Cities15 writes:

1. Fresh-water-housing bool and urban-defenses i32.
2. Home Continent hash and counted settled Continent hashes.
3. Peak-Cities u32, extra starting population off home continent i32, population after conquest i32.
4. Counted banned Building, District, District-building and Project hashes, in that order.
5. Counted Building-to-Building override hash pairs, preserving duplicates and order.

The unused `m_buildingBuildMap` is not invented as another wire vector. `m_Initialized` is likewise not serialized by this writer. Native names are taken from matching Linux layout leads and direct member-address/call observations; omitted live values are not classified as planning-only.

The FAutoVariable wrappers dispatch to virtual save slot `+0x28`. Exact ELF relocation bindings for tables (including the 16-byte Itanium header) are:

| Table | Concrete save |
|---|---|
| `0x3ad5e48` (Continent) | `0x2a60600` |
| `0x3ad5f08` (Continent vector) | `0x2a61060` |
| `0x3ad5fc8` (int) | `0x2a61f70` |
| `0x3ad5cc8` (Building vector) | `0x2a5e5f0` |
| `0x3ad6088` (District vector) | `0x2a629a0` |
| `0x3ad6148` (Project vector) | `0x2a63880` |
| `0x3ad5d88` (Building-pair vector) | `0x2a5f400` |

Concrete sequence writers are Continent `0x2a61bc0`, Building `0x2a5f020`, District `0x2a634b0`, Project `0x2a643b0`, and Building-pairs `0x2a60110`; pair writer `0x2a60200` writes two Building enums. Continent scalar writer `0x2d4efd0` calls GetContinentDefinitions then WriteIndexToHash. No registry resolution is performed by the Rust wire reader.

## Bounds and verification

One aggregate byte/item budget spans Religion, both recycled collections, all nested City components and manager tail vectors. Child decoders receive remaining credits and return exact item use; counted names charge byte items. All explicit vector allocations use checked element-size multiplication and fallible reserve. This module follows existing `unit::Limits`/City policy: aggregate items and checked per-vector allocation bounds imply a finite item-derived allocation bound, but it does **not** report Map's separate exact logical-allocation metric or claim one shared allocation-byte cap over all City children. The auxiliary duplicate-slot list is bounded by the already charged live count. Fixed arrays and native pointers are not fabricated.

Six source-only tests cover nonempty Religion vectors/raw names, empty Cities with complete nonempty allocator metadata and tail vectors, duplicate override preservation, exact cumulative byte/item budgets, every truncated prefix, untouched suffixes, empty Manager, mismatched owner/noncanonical bool rejection, explicit slots/duplicate refusal and nonempty Formation path/member framing delegated to its existing codec. Encoder tests additionally check exact prefix/item roundtrips, zero-budget empty Manager framing, byte-budget refusal and inconsistent partial-prefix models. These are synthetic grammar tests, not native gameplay fixtures.

An offline separate Rust harness read both existing immutable native Core streams (`ff31f4c…d884` and `85d858ef…87a8`). The existing structural Map parser supplied the validation cursor; no fixture offsets, traces or byte-pattern searches enter the new decoder. Both decode **211,525 post-Map bytes, 28,030 dynamic items, 43 Manager entries, 35 first-player Formations and 11 complete first-player Cities**. Adding the independently computed Map end gives absolute stop **4,907,389**, which matches the native ownership trace's first Player.Cities end and Player.Diplomacy start. Both entire **211,525-byte** typed prefixes re-encode byte-identically, including all Cities and trailing allocator/manager metadata, with encoder and decoder agreeing on the **28,030-item** charge. Exact cumulative byte/item budgets accept and one-less budgets refuse on both fixtures. These numbers are observed validation results, not constants in parser decisions. Both streams describe the same fixture context, not independent gameplay branch coverage.

Complete logical gameplay state, all gameplay RNG and policy decision boundaries remain required work. The recorded **89,198-byte raw Save→Load difference remains unmasked**. This bounded wire milestone makes no state/action/turn parity claim.
