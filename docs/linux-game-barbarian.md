# Linux Barbarian Manager current wire

`game_barbarian` transports the current `GameCore::Barbarian::Manager` v5 record of the Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. This preserves NPC and gameplay-affecting state regardless of its namespace. It does not execute upkeep, native Read, player policy, initialization, definition lookup, actions or turns.

The outer writer/reader are `0x282d310` / `0x282d410`. Game Write calls it at `0x2d7a889` after GlobalAi; Game Read calls it at `0x2d7abaa`. All words are little endian. Player and control-priority enums remain raw i32 ordinals. Tribe-name, Improvement, Tech and Unit definitions remain u32 hashes, including unknown/duplicate hashes. Bool bindings transfer a raw byte; the transport preserves every carrier value. No serialized field here uses native float or Q8 conversion.

| Current record | Exact wire order |
| --- | --- |
| Manager v5 | player ordinal; three wrapped u32 values (object fields `0xc8`, `0xe0`, `0xf8`); control-priority ordinal; Improvement hash; raw bool at `0x168`; counted i32 vector; counted Instance vector; counted CampLocation vector; ClansManager |
| Instance v15 | player ordinal; tribe-name hash; tribe index; PlotCoord; raw living bool; rampage-target ordinal; six i32 values at `0x20..0x34`; three i32 values at `0x80..0x88`; four required-spawn i32 values at `0xa8..0xb4`; counted ASCII string vector; three behavior-tree u32 IDs; two counted ComponentID vectors; raiding score; turns in enemy territory; TypedVariantMap; counted Tech hashes; steal cooldown; conversion points; counted bribe-turn i32 vector; counted ClanPlayerStats vector; counted WeightedPlayer vector |
| ClanPlayerStats v15 | five action counts i32; counted Unit hashes hired; five last-action-turn i32 values |
| CampLocation v1 | plot index i32 |
| ClansManager v3 | counted UniqueUnitClaimData vector; i32 at `0x100` |
| UniqueUnitClaimData v3 | Unit hash; two i32 values |
| WeightedPlayer | player ordinal i32; weight i32 |
| ComponentID | owner i16; component type i16; ID u32 |
| PlotCoord | x i32; y i32 |
| ASCII string | u32 byte count; exactly that many bytes, without a serialized terminator |

Instance and ClanPlayerStats share the native version constant 15 at `0x2f19340`. ClansManager and UniqueUnitClaimData share constant 3 at `0x2eec340`. These are actual writer constants rather than inferred historical class ages. The current writer does not serialize Instance's local food-yield member at `0x1a8`; its absence is not a zero-value assumption. Historical reader branches are rejected by this current-wire codec.

Properties reuse the existing bounded `unit::variants::TypedVariantMap` typed transport and its explicit unsupported-type/modifier/normalization errors. No opaque property byte range is retained. Canonical records have empty properties; synthetic tests include a named Char8 property. This does not expand the variant helper's supported branches or establish live property behavior. Consequently, every current structural field and both complete observed outer records are supported, while arbitrary current-wire coverage remains incomplete for unsupported typed-property branches. The audit also gates host `Civ6` SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4` and the existing Linux variant method fingerprints. It pins original serialization methods and DWARF layouts; all method identities are recorded in its JSON result.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode`, and `encode_with_items` use `unit::Limits`. Actual wire bytes and aggregate counted items are charged across nested containers, strings and properties. Counts are checked against minimum remaining wire, shared item credit, checked owned allocation sizes and fallible vector reservation. Exact caps pass; one-short byte/item caps fail. Offsets in this document and trace extents are comparison evidence only and never parser inputs.

The read-only audit independently projects every concrete field in both native records, reconstructs all bytes, compares every native Instance cut, and checks the decoded Rust field projection against that independent projection. It builds an unchanged, exported, committed Git archive, verifies all source files against Git before and after, runs the actual module Cargo tests and strict Core Clippy, and checks full roundtrips, exact shared caps, strict trailing rejection, prefix consumption, hostile counts/versions and bounded native truncations. Small synthetic carriers test all 256 raw bool values, all truncations, nonempty nested records, hash duplicates, byte strings and properties. Missing crate exports fail; source injection is prohibited.

Both canonical fixtures contain the same 28,817-byte Manager record with 704 aggregate items and 197 native Instance children. Camp locations, ClanPlayerStats and unique-unit claims are empty in those fixtures and are exercised by synthetic carriers. Comparison extent: `70934595..70963412`; wire SHA-256 `22b8e191c94faa59a4c992b9404282bbd0d21b0f371c69ef51bab5ae25301185`. The next Core owner is `GameCore::FreeCities::Manager` at `70963412..71027778`.

Native Read resolves player pointers and hashes, initializes signals and camp evaluators when a player exists, initializes/resizes legacy data, and edits archive-backed variables. Those live effects are outside this transport. No unencoded live context is guessed. No state is classified as planning-only. Future gameplay must execute exact rules/upkeep and yield to external policies for every player decision.

This is a bounded wire component. Full Player/world import, initialized-game acceptance, gameplay state/RNG, actions and turns remain unsupported. The recorded raw Save→Load difference remains **89,198 bytes**, unmasked and unclassified. See [the audit evidence](../evidence/linux/game-barbarian.json) for the immutable candidate and validation identities.
