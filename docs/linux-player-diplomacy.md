# Linux Player.Diplomacy wire component

`player_diplomacy::Diplomacy` reads and writes the current version-50 serialized
component from the pinned Linux XP2 target. It preserves every serialized field,
including diplomacy state whose enum names belong to the AI namespace. This is a
wire codec, not a native object constructor, diplomacy rules implementation,
complete Player importer, action executor or turn engine.

The target is `libGameCore_XP2.so` SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, with host
SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The complete outer writer is `0x2a9c520` (1,165 bytes); the reader is `0x2a9c9b0`
(4,513 bytes). `Serialization::CurrentVersion` at `0x2f59980` is 50. The public
audit fingerprints complete method bytes, direct call bindings, and the concrete
FAutoVariable save/load vtable slots. Matching target DWARF checks field identity.

## Current wire grammar

Integers and enum words occupy four little-endian bytes, bools one canonical
byte. Native registry index serializers produce definition hashes; those remain
hashes in the typed codec. Fixed-point values retain their signed raw words.
All counted containers use u32 counts. Fixed arrays have exactly 64 elements and
no count prefix. The exact outer order is:

1. Version 50; friendly-meeting, delegation and embassy bool arrays; two open
   border int arrays; enforce-borders bool; five denouncement/city/culture-bomb/
   nuke int arrays; converted-city and occupied-city bool arrays; offered-deal,
   war-weariness, warmonger and era-met int arrays.
2. Two counted ordered yield-hash/int lists. Ten war/alliance scalar int words,
   warmonger fixed-point word, no-occupation-penalties bool.
3. The complete independently versioned
   [DiplomacyBlackboard](linux-player-diplomacy-blackboard.md), followed by counted
   `(u32 city ID, i32 weariness)` records and 64 independently versioned
   CasusBelliData records.
4. Counted yield, movement, combat, action prerequisite override and banned
   action/civilization modifier records.
5. Ten last-war/city/count int words; shared-alliance-visibility bool, grievance
   generation and decay ints, disabled-decay bool; nine alliance/favor int words.
6. Counted visibility enum words, visibility source hashes and visibility
   addition records. Counted player/gossip maps; held messages; promises made;
   incursions made; last promises made.
7. Counted unit-hash/bool pairs, alliance points int vector and diplomatic-action
   hash/int pairs.

The three grouped scalar arrays retain the writer order: `war_and_alliance_modifiers`
is enemy/domestic/overall weariness multiplier, joint war experience range,
plunder multiplier, allied warmonger discount, maximum warmonger percent, alliance
points for modifier/trade/common enemy. `last_war_and_city_turns` is war initiated,
war received, city liberated, surprise war, liberation war, protectorate war,
territorial war, city captured, total wars and surprise wars declared.
`alliance_and_favor_modifiers` is science, tourism, culture, faith per citizen,
religious pressure, envoys, research turns, favor per delegation and embassy.

Nested record framing is explicit:

| Record | Version | Ordered body |
|---|---:|---|
| CasusBelliData | 50 | Nine bools: colonial, holy, liberation, protectorate, reconquest, territorial, golden age, retribution, ideological war |
| PromiseData | 50 | Turn made; skipped, broken, notification sent, repeat offense bools; turn broken |
| MessageData | 1 | Kudo bool, hidden-agenda bool, player word, message hash |
| VisibilityAddition | 50 | Amount, source hash, source-type word |
| YieldModifier | 2 | Source word, turns, yield word, modifier, stack bool |
| MovementModifier | 1 | Source word, turns, extra moves |
| CombatModifier | 1 | Source word, turns, combat bonus, religious-only bool |
| ActionPrereqOverride | 1 | Action hash, civic hash, tech hash |
| BannedActionCivilization | 1 | Action hash, civilization hash |

Each inner gossip, promise and incursion `vector_map` writes its own version 1
before its count. `TypeMap` preserves that framing even for empty maps. Outer
player maps and all inner maps preserve order and duplicate keys; encoding never
sorts or deduplicates. IncursionData has one unversioned int word. No SQL ordinal
or registry mapping is inferred from the serialized words.

## Bounds and application limits

`decode_prefix_with_items` returns `(value, consumed_bytes, dynamic_items)`;
`encode_with_items` returns `(bytes, dynamic_items)`. Limits aggregate every
counted container entry, nested map entry and Blackboard string byte. Fixed arrays
consume byte budget but no dynamic item charge. Composition must subtract the
returned charges before parsing its next component. Count and allocation checks
precede reserve; unsupported versions and noncanonical bools fail explicitly.
The strict decoder rejects trailing bytes, while the prefix decoder leaves them
for the next source-derived component.

Native Read invokes Reset, registry lookup/remapping and version-dependent
application behavior. This codec does not claim to reproduce those operations,
constructor defaults, caches or missing-field initialization. The unit-entry and
action-count hash arrays remain ordered wire overlays; their native destination
sizes/defaults are separate work. Historical outer/nested versions are rejected.
No gameplay data is excluded because a C++ type belongs to an AI namespace.

## Validation

Run `python3 tools/linux/player_diplomacy_audit.py --output evidence/linux/player-diplomacy.json` from the repository root. The audit checks
both pinned original canonical streams against their historical owner traces,
uses those traces only to locate validation records, then pipes each record into
the actual Rust codecs and compares every encoded byte and consumed length.
There are no production offsets, owner-trace lookups or byte-pattern decisions in
the codec. Raw proprietary bytes and disassembly/DWARF dumps are never written by
this audit. It runs an isolated Rust candidate and does not rebuild the parent
Core candidate or execute original game code. Its temporary harness includes the
outer module by path and imports the real crate's `unit` and Blackboard modules.

The recorded audit covers 43 complete Diplomacy records (970,209 bytes) per
canonical stream, 86 records and 1,940,418 compared bytes in total. Each stream
contains nonempty gossip, promise and incursion maps, 17 banned action/civilization
records, 39 visibility additions, 4,988 unit-hash pairs and 1,806 action-hash pairs.
Yield/movement/combat modifiers, prerequisite overrides and held messages are
empty in these two original fixtures; their nonempty branches have source tests
and static target contracts, without a nonempty original fixture comparison.

Source tests cover nonempty nested maps/vectors, duplicate keys, signed boundaries,
raw hashes, nested version refusals, every byte truncation of a nonempty record,
invalid bools, trailing bytes, malicious counts, exact aggregate limits and both
one-short budgets. Offline wire roundtrips do not establish native Read parity or
live diplomacy state parity. Complete gameplay state, actions and turns remain
unimplemented, and the recorded raw 89,198-byte Save→Load difference remains an
unmasked failure.

The next genuine unframed Player dependency after this component is
`GameCore::Player::Districts`. On the historical first validation record, Diplomacy
occupies `[4907389, 4956330)` and Districts starts immediately afterward. These
locations are observations, not parsing constants.

Discovery attribution: cru121, *Civ6-GameCore-reference*, commit
`5af389445118bed2bdebf7a8a42b219c8fba5175`, particularly its Linux DWARF layout
metadata, under [CC BY 4.0](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/LICENSE).
The supplied Linux target's actual bytes and DWARF provide the contract checks;
upstream metadata alone is not evidence of cross-platform equality.
