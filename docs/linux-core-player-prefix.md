# Linux post-Map Player prefix

[`core_player::Prefix`](../crates/c6-core/src/core_player.rs) reads the current Linux raw Core wire stream beginning immediately after Map. It decodes Game.Religion7 and every ReligionData7, Player.Manager1's framing, the first Player.Instance47 header, the complete first Formation collection, Goody_Hut1, Bonuses4, Congress12 and **the complete Player.Cities15 collection and its trailing fields**, followed by complete Player.Diplomacy50, Player.Districts8, Player.NameManager2 and Player.Culture45. It stops before unframed Player.Espionage. It does not finish the first Player, reach the remaining Players, establish a full City/Formation/Unit roster, load a live game or execute actions/turns.

`Prefix::decode_prefix_with_items(bytes, unit::Limits)` returns `(typed_prefix, consumed_bytes, dynamic_items)`; `decode_prefix` omits the item charge. The consumed position identifies a prefix boundary, not exhaustion of a complete Player record. `Boundary::PlayerEspionage` and `unfinished_player_entries` explicitly retain that distinction: for a nonempty Manager the unfinished count includes the partially read first entry. Empty Managers return `AfterEmptyPlayerManager` without consuming later Game components. Trailing input is untouched. `encode_prefix_with_items` returns the encoded supported prefix plus its exact item charge; `encode_prefix` omits that charge. Encoding refuses inconsistent boundary/count/first-entry combinations and writes no unsupported tail. There is intentionally no strict full-Player decoder or opaque unframed skip.

Manager's requested owner and the record's embedded owner remain separate fields. The original Manager writer obtains the ID from the same Player it then serializes; this supported writer-wire grammar requires equality and rejects a mismatch. It does not infer a roster-index/player-ID mapping or reproduce Manager.Read initialization and fixups. The header retains its fifteen ordered i32/enum scalars and six bools without speculative scalar names; hashes remain full u32 words, and religion names are counted byte strings without UTF-8 normalization. Formation and City owners are preserved as written without rewriting them to the Manager ID.

## Exact source contracts

The target is Linux Core SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, with matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. Existing [Core layout](linux-core-stream-layout.md), [Player components](linux-player-components.md) and [complete City wire evidence](linux-city-wire.md) supply the accepted preceding/current child grammars. Original execution remains identity/configuration gated; this implementation does not execute original code. Reference layout leads use cru121's pinned revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, CC BY 4.0, with [attribution and limitations](reference.md).

Relevant original writer/reader addresses are Religion `0x2a157c0`/`0x2a15820`, ReligionData writer `0x2a15560`, Player.Manager `0x2b51060`/`0x2b510f0`, Player.Instance `0x2b4ee00`/`0x2b4f490`, and Player.Cities writer `0x2a579d0` (231 bytes). These are source evidence, never parser offsets. The existing Formation strict codec validates the exactly sized current record: fixed 36-byte prefix, count plus 20-byte PathNode records, cached coordinate and counted u32 member IDs. Those counted lengths determine its slice dynamically.

City recycled-container writer `0x2a5b840` (374 bytes, existing checked SHA-256 `2819e602eddaaf3faebfcdc440a62e75550c30d02047c90cc976adb0b81a41bc`) writes version65537, index-bits16, capacity, live count, then each **explicit signed slot plus complete City51**. The trailing metadata is current generated-ID bits, free count, free head, free tail, then exactly free-count u32 links **without another count prefix**. The same framing is used for Formations. Entry order and allocator words are retained; duplicate/out-of-range live slots reject, and generated IDs are never used to synthesize slots. Zero live entries still consume all allocator metadata and manager tail fields.

After this container, Player.Cities15 writes:

1. Fresh-water-housing bool and urban-defenses i32.
2. Home Continent hash and counted settled Continent hashes.
3. Peak-Cities u32, extra starting population off home continent i32, population after conquest i32.
4. Counted banned Building, District, District-building and Project hashes, in that order. Both District vectors—including native `m_bannedDistrictBuildings`—contain `GameCore::DistrictTypes` definition hashes; the field name does not change the second vector to BuildingTypes.
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

## Historical Cities-only bounds and verification

One aggregate byte/item budget spans Religion, both recycled collections, all nested City components and manager tail vectors. Child decoders receive remaining credits and return exact item use; counted names charge byte items. All explicit vector allocations use checked element-size multiplication and fallible reserve. This module follows existing `unit::Limits`/City policy: aggregate items and checked per-vector allocation bounds imply a finite item-derived allocation bound, but it does **not** report Map's separate exact logical-allocation metric or claim one shared allocation-byte cap over all City children. The auxiliary duplicate-slot list is bounded by the already charged live count. Fixed arrays and native pointers are not fabricated.

Six source-only tests cover nonempty Religion vectors/raw names, empty Cities with complete nonempty allocator metadata and tail vectors, duplicate override preservation, exact cumulative byte/item budgets, every truncated prefix, untouched suffixes, empty Manager, mismatched owner/noncanonical bool rejection, explicit slots/duplicate refusal and nonempty Formation path/member framing delegated to its existing codec. Encoder tests additionally check exact prefix/item roundtrips, zero-budget empty Manager framing, byte-budget refusal and inconsistent partial-prefix models. These are synthetic grammar tests, not native gameplay fixtures.

An offline separate Rust harness read both existing immutable native Core streams (`ff31f4c…d884` and `85d858ef…87a8`). The existing structural Map parser supplied the validation cursor; no fixture offsets, traces or byte-pattern searches enter the new decoder. Both decode **211,525 post-Map bytes, 28,030 dynamic items, 43 Manager entries, 35 first-player Formations and 11 complete first-player Cities**. Adding the independently computed Map end gives absolute stop **4,907,389**, which matches the native ownership trace's first Player.Cities end and Player.Diplomacy start. Both entire **211,525-byte** typed prefixes re-encode byte-identically, including all Cities and trailing allocator/manager metadata, with encoder and decoder agreeing on the **28,030-item** charge. Exact cumulative byte/item budgets accept and one-less budgets refuse on both fixtures. These numbers are observed validation results, not constants in parser decisions. Both streams describe the same fixture context, not independent gameplay branch coverage.

Complete logical gameplay state, all gameplay RNG and policy decision boundaries remain required work. The recorded **89,198-byte raw Save→Load difference remains unmasked**. This bounded wire milestone makes no state/action/turn parity claim.

## Historical Cities-only direct byte-zero library composition

`core_stream::ThroughFirstPlayer::decode` now composes the accepted raw Core prefix/Map reader with this module. It receives the entire decompressed raw Core buffer, derives the Map boundary itself and parses the supported Player prefix from that cursor. No Python offsets, native owner traces or searches are inputs to the library.

```rust
use c6_core::{core_stream::ThroughFirstPlayer, map, unit};

let decoded = ThroughFirstPlayer::decode(
    &raw_core,
    map::Limits::default(),
    unit::Limits::default(),
)?;
let first = decoded.player_prefix.first_player.as_ref();
let remaining = decoded.tail; // borrowed at the explicit partial boundary
```

The result retains the typed initial Core prefix in `core`, the new `player_prefix`, aggregate `items_used`, absolute `consumed` and a final borrowed `tail`. `core.tail` still denotes the original Map-end boundary; `decoded.tail` denotes the later supported boundary. The shared Map-policy byte/item ceilings span both portions; the additional Player ceilings intersect their remaining credits. Map's `max_allocation` and variant-depth policy apply through Map. Later City/Player components retain their independently documented byte/item and checked per-vector allocation policy; this API does not report a fictitious exact combined allocation charge.

The [whole-input audit](../tools/linux/core_player_prefix_audit.py), [offline driver](../crates/c6-differential/src/bin/c6-core-player-prefix-candidate.rs) and [result](../evidence/linux/core-player-prefix.json) validate both original inputs. The driver re-encodes all typed post-Map data—including every first-player City—and compares the **entire** consumed 211,525-byte region. All Religion fields, ordered Player header words/flags, Formation/City slots and allocator metadata, City identities/hashes and trailing Cities vectors are independently checked. Existing native City cuts are used only by the independent comparison, never by the Rust parser. The trace-independent original-boundary evidence remains distinct from executing native `Read` fixups.

The absolute consumed boundary is **4,907,389** on both originals; the remaining original tail is **75,750,073 bytes**. All 43 Manager entries are still unfinished because the first entry stops at Diplomacy. Source-only cases insert 20 bytes of framed Context metadata and one extra four-byte settled-Continent entry; the whole-input decoder relocates subsequent boundaries and preserves all untouched tail bytes. No insertion is claimed to be a native gameplay fixture.

Reproduce using:

```sh
python3 tools/linux/core_player_prefix_audit.py --frozen-profile target/frozen/core-player-prefix/2578ba02fc7c001cd8dfc1cafe39f33de064194e04f08bebdfa4640ba1e93742 --output target/linux/core-player-prefix.json
cargo test -p c6-core core_player
cargo test -p c6-core core_stream
```

The workspace tests, strict workspace Clippy and C ABI smoke pass for this integrated checkpoint. This is original wire-data comparison with an offline Rust executable, not original-code execution. The next unsupported unframed component is **Player.Diplomacy**; completing remaining Players and reaching their District/Unit rosters requires their intervening real schemas. Runtime Manager validity/65-slot mappings, Read fixups, complete logical state and RNG are not reconstructed by Manager wire membership alone.

Independent review [592d660](linux-core-player-prefix-review.md) accepted the immutable `2578ba02fc7c001cd8dfc1cafe39f33de064194e04f08bebdfa4640ba1e93742` candidate and its 51 tested source files. It confirmed both original projections/whole-prefix roundtrips and owner boundaries, all 213 method fingerprints/seven concrete bindings, and budget/model rejection cases. The partial import and runtime-state limitations remain unchanged.


## Historical extension through Diplomacy and Districts

The current `FirstPlayer` adds typed `diplomacy: player_diplomacy::Diplomacy` and `districts: player_districts::Districts` immediately after Cities. The direct existing child codecs supply exact prefix lengths and item charges in both directions; their remaining byte/item credits come from the same aggregate prefix budget. No new stage wrapper, inferred skip, hash normalization, policy choice or live-state default is introduced. The first partial Player now stops explicitly at `Boundary::PlayerNameManager`. Empty Manager behavior remains `AfterEmptyPlayerManager`, and the unfinished roster count still includes the first partial entry.

The complete current schemas and original evidence are documented in [Player Diplomacy](linux-player-diplomacy.md), [Diplomacy Blackboard](linux-player-diplomacy-blackboard.md) and [Player Districts](linux-player-districts.md). Original Player writer order is Cities, Diplomacy, Districts, NameManager. Diplomacy includes its complete nested Blackboard; Districts includes its complete recycled District roster, allocator metadata and manager fields. These are serialized wire values, not reconstructed runtime ownership, native Read fixups, diplomacy decisions or executable gameplay.

The six scoped source tests now exercise the larger boundary. Their synthetic carrier adds nonempty nested PromiseData/Incursion TypeMaps, full-width visibility hashes, an empty actual District roster with nonempty allocator links, duplicate indexed District hashes and a nonempty versioned DistrictCreateYield with raw Yield index and bool fields. Remaining nested collections are explicitly empty according to their source framing. These are legitimate wire carriers and make no claim about native initialization defaults. Full-prefix truncation, unchanged suffix, exact aggregate item/byte boundaries, re-encoding and model/identity rejection remain checked.

A separate offline Rust harness parsed and byte-identically re-encoded both immutable original streams through **292,438 post-Map bytes**, with exact encoder/decoder agreement at **43,794 dynamic items**. Exact cumulative budgets accept; one-less byte/item budgets refuse. The independently derived absolute stop is **4,988,302**, at first Player.Districts end / Player.NameManager start. Both fixtures still have 43 unfinished Player entries, 35 first-player Formations and 11 first-player Cities. The historical Cities-only candidate `2578ba02fc7c001cd8dfc1cafe39f33de064194e04f08bebdfa4640ba1e93742`, its report and review remain immutable evidence for their earlier stop; these new results are a distinct extended profile, whose reproducible integrated audit is maintained by the coordinator.

No original code executes for this integration. Full Player/world import, native state normalization, complete gameplay state/RNG, action execution and turns remain unsupported; the 89,198-byte raw Save→Load difference remains unmasked.

The current whole-input profile is documented in [Core prefix through first Player Districts](linux-core-player-districts-prefix.md). The command above now explicitly replays the earlier immutable Cities-stop candidate and checks every archived source hash; it does not rebuild the advancing current API under the old boundary label.


## Current extension through NameManager and Culture

`FirstPlayer` now adds typed `name_manager: player_name_manager::NameManager` and `culture: player_culture::Culture` after Districts. Decoding and encoding invoke the complete current child codecs with the same aggregate remaining byte/item credits; `Boundary::PlayerEspionage` identifies the next unsupported unframed component. The Manager-empty path remains unchanged, and all 43 entries in the original fixture remain unfinished. The prior Districts-stage candidate `5edcd6df…` and its report remain historical evidence for their earlier boundary; the coordinator maintains a distinct extended audit profile.

The [NameManager wire contract](linux-player-name-manager.md) preserves its equal explicit signed length and WriteArray length, followed by ordered i16 usage counts. The [Culture wire contract](linux-player-culture.md) closes all 63 current outer fields and eleven nested record types, including double-length GovernmentSlots, unversioned tourist/location entries and explicitly versioned v45 records. Registry hashes, raw ordinals, scalar bit patterns, nested ordering and duplicate keys remain unchanged. No runtime constructor defaults, policy choices or native Read/configuration fixups are inferred.

`PlayerHeader::saved_is_alive()` returns exactly `ordered_flags[0]`, the saved Player47 `m_bIsAlive` boolean. [The exact Linux header-control evidence](linux-player-header-control.md) establishes writer, reader, wrapper and getter identity. This accessor does not establish initialized slot validity, actual configuration, loaded-world validity or post-Read state; Team and Human remain external configuration dependencies. No Player/world installation or turn advancement is implemented.

Seven scoped source tests exercise both complete new grammars. The arbitrary typed carrier uses three signed NameManager counts, all Culture fields and all eleven nonempty nested record forms, duplicate prerequisite hashes, raw scalar extremes, nested tourist counts and explicit GovernmentSlot ordinals. It roundtrips through the whole supported prefix, preserves the Espionage suffix, accepts exact combined source budgets and refuses one-less budgets and every truncation. A separate combined test toggles only the first saved flag, roundtrips its value, rejects mismatched NameManager double lengths and rejects an unsupported Culture version.

A separate offline Rust harness byte-identically re-encodes both original prefixes through **299,432 post-Map bytes**, with exact encoder/decoder agreement at **45,098 dynamic items**. Both original prefixes accept exact cumulative byte/item caps in decoding and encoding and reject one-less caps; the child per-vector allocation policy remains explicit. The computed absolute stop is **4,995,296** (first Culture end), and the next serializer is Player.Espionage. These are validation observations, never parser constants. No original game code executes; complete Player/world import, logical gameplay state/RNG, actions and turns remain unsupported. The recorded 89,198-byte raw Save→Load difference remains unmasked.
