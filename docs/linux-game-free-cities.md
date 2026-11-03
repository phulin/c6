# Linux FreeCities Manager current wire

`game_free_cities` transports every current structural wire field of `GameCore::FreeCities::Manager` v3 and its Instance v5 children for Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. It preserves gameplay-affecting NPC state; it does not execute native Read, initialization, lookup, events, upkeep, policy, actions or turns.

The current Manager writer/reader are `0x294d890` / `0x294d930`; Instance writer/reader are `0x294b990` / `0x294ba50`. Game Write calls Manager at `0x2d7a899`, after Barbarian and before sync Random. All scalar words are little endian.

| Record | Exact current wire order |
| --- | --- |
| Manager v3 | version u32 = 3; player ordinal i32; control-priority ordinal i32; u32 Instance count; those Instance records |
| Instance v5 | version u32 = 5; player ordinal i32; city index i32; city location x i32, y i32; raw living bool u8; turns since last unit i32; urban defenses i32; retaliation-target player ordinal i32; retaliation behavior-tree ID u32; Unit hash/bool list; Building hash/bool list |
| Each hash/bool list | u32 count; that many ordered pairs of definition hash u32 and raw bool u8 |

No current field requires strings, float or Q8 conversion. Player and control priority retain raw ordinals. Bool bindings transfer one raw byte (`WriteObject(bool)` at `0x1fe7900`, `ReadObject(bool)` at `0x1fe81a0`) without logical normalization. Unknown and duplicate hashes, original order and all 256 bool storage carriers are preserved.

Unit list writer `0x294d020` calls `WriteIndexHashArray<bool, Unit>` at `0x294d310`; Building list writer `0x294d060` calls its Building counterpart at `0x20031e0`. These emit the definition collection count and then each definition hash plus its byte. The saved wire is self-delimiting; it requires no guessed live registry context. Native replay into a definition-indexed list is a separate operation: the readers at `0x28898d0` and `0x2002f30` remap hashes and consume unknown hashes into scratch storage. This codec neither remaps nor drops saved entries. Native writer reachability with a particular rules database is not inferred from transport acceptance of a list.

The Instance DWARF layout names `m_cityIndex`, `m_cityLocation`, `m_living`, `m_turnsSinceLastUnit`, `m_iUrbanDefenses`, `m_retaliationTarget`, `m_retaliationBehaviorTree`, `m_buildableUnits`, and `m_buildableWalls` at object offsets 8, 12, 20, 24, 28, 32, 36, 40 and 72. Manager DWARF places `m_priority`, `m_freeCities` and `m_player` at 176, 200 and 240. These object offsets explain code bindings; they are never serialized-position inputs. The audit pins actual serialization methods, call relocations and the DWARF dump identity against the complete module SHA.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode`, and `encode_with_items` use `unit::Limits`. Shared item credit counts the Instance vector and every Unit/Building entry across all children. Wire credit covers every consumed/emitted byte. Decode checks count multiplication, remaining minimum wire, aggregate credit, checked owned allocation sizes and fallible reservation before constructing each vector. Historical versions are explicit errors. Unsupported operations do not mutate a live game.

Both canonical native records have comparison extent `70963412..71027778`: 64,366 bytes, 55 native Instance children, 6,380 Unit hash/bool entries and 5,995 Building hash/bool entries, totaling 12,430 items. Every direct and transitive native owner cut is compared against the independent typed parser; no deeper unverified owner exists in the observed record. The next Core owner is `GameCore::Random` at `71027778..71027796`. Trace offsets select test records only and never drive the production parser.

The audit independently projects all fields, reconstructs every native byte, then compares Rust decoded field projections and complete roundtrips for both outer records. It requires the actual driver and exported crate from an unchanged committed Git archive, verifies every source input against Git before and after building, runs actual module Cargo tests and strict all-target Core Clippy, and rejects missing exports. It never injects source into an archive. Native cuts, bounded truncations, hostile versions/counts, exact and one-short shared caps, prefix consumption and trailing rejection are checked. Small synthetic carriers additionally cover every bool byte, every small truncation, empty containers, extreme ordinals and duplicate/unknown hashes. No original code is executed, and no private artifact is written.

Native Manager Read resolves/validates its player, edits archive-backed variables and initializes signals; Instance Read resolves player pointers and resets/remaps definition-indexed lists. Historical reader branches synthesize legacy defaults, which this current-version wire codec rejects. These live effects remain outside the transport. No state is classified as planning-only: exact NPC upkeep, gameplay RNG and all gameplay-affecting state remain in scope, and every player decision must yield to an external policy.

This is bounded state transport, not full Player/world import or actual save loading. Full gameplay state/RNG, native Read, actions and turns remain unsupported. The recorded raw Save→Load difference remains **89,198 bytes**, unmasked and unclassified. The immutable validation identity is recorded in [the evidence](../evidence/linux/game-free-cities.json).

Validation used unchanged exported source archive `194711e`. All three actual module Cargo tests, strict all-target Core Clippy, and both complete Rust field-projection/roundtrip comparisons passed. All 108 source inputs matched Git before and after the build. Reproduce with no original execution or private writes:

```sh
python3 tools/linux/game_free_cities_audit.py \
  --source-commit 194711e --output /tmp/game-free-cities.json
```
