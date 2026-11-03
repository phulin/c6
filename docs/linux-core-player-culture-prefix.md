# Direct Core prefix through first Player Culture

`core_stream::ThroughFirstPlayer` now reads raw Core bytes from byte zero through the first Player's complete Culture component. It composes the earlier supported prefix with NameManager2 and Culture45. The explicit next boundary is **Player.Espionage**; all 43 Player entries remain unfinished. This is a typed wire import, not complete world loading, native `Read` application or gameplay.

The separate [driver](../crates/c6-differential/src/bin/c6-core-player-culture-prefix-candidate.rs), [audit](../tools/linux/core_player_culture_prefix_audit.py) and [report](../evidence/linux/core-player-culture-prefix.json) define this profile. Candidate SHA-256 is `7c563c56970ee8fcdfe6c0761d091a5361f909c6f95ce1d8415dc62cb2a2b8f0`; its executable, report and 61 exact source files are frozen locally under `target/frozen/core-player-culture-prefix/7c563c56970ee8fcdfe6c0761d091a5361f909c6f95ce1d8415dc62cb2a2b8f0-69ce855cac97/`. Previous Cities/Districts profiles retain their own immutable candidates and reports.

Both existing original 80,657,462-byte Core streams produce:

| Observation | Result |
|---|---:|
| Complete Map end | 4,695,864 |
| Complete post-Map prefix | 299,432 bytes / 45,098 items |
| Absolute consumed cursor | 4,995,296 |
| Total charged items | 2,003,194 |
| First Player Formations / Cities / Districts | 35 / 11 / 72 |
| NameManager | 92 bytes / 40 signed counts |
| Culture | 6,902 bytes / 1,264 items |
| Acquired-civic wire pairs | 61 |
| Untouched borrowed tail | 75,662,166 bytes |

The complete post-Map region re-encodes exactly, SHA-256 `6c666ec0f0d4c91417239a5c4422642111cf4a7fe7cb6b411600d3c1465aee5c`. The audit independently checks NameManager's two equal counts and signed i16 values, Culture's acquired-civic projection after its government/policy collections, the saved-alive flag and all earlier projections/record hashes. Native owner cuts are validation-only inputs to the independent comparison; the Rust library and candidate receive the complete raw Core with no offsets or traces. The native Culture end and Espionage start both match the dynamically computed cursor. No NameManager entry hook is invented for the historical trace.

NameManager's current writer emits vector count plus `WriteArray` count; historical v1 and configuration-dependent `PadNameUsageCount` application remain unsupported. Culture preserves all 63 fields, eleven nested record types, policy slot ordinals, definition hashes and raw Q8 words. Its acquired-civic data uses the existing `IndexedBools` type. Actual definition lookup and the proven owner Reset/overlay helper remain separate from wire parsing.

A full-prefix exact-budget regression exposed an unintended cumulative heap cap in Culture. It was corrected before this freeze: consumed bytes/items remain aggregate, while typed allocations follow the existing checked per-vector `unit::Limits` policy. All 86 original Culture records pass exact byte/item decode and encode caps; one-short caps reject. The full 299,432-byte prefix passes the same exact/one-short tests. No synthetic runtime zeros were introduced.

`PlayerHeader::saved_is_alive()` exposes verified `ordered_flags[0]`. Original writer, reader and getter use the same `m_bIsAlive` byte; the [source contract](linux-player-header-control.md) pins that bridge. Both existing first headers save true. This does not supply global player-slot validity, Team or human status: Team/SlotStatus need actual configuration. Player.Diplomacy data must not substitute for the separate Game.DiplomaticRelations authority.

The API still borrows its unparsed tail and carries shared byte/item credits through every child. Map allocation/depth policy applies through Map; later components retain their documented checked per-vector allocation policy. No whole-world clone, opaque unframed skip, definition-index guess, metadata compatibility check or original simulation execution occurs.

Replay the immutable local profile with:

```sh
python3 tools/linux/core_player_culture_prefix_audit.py --frozen-profile target/frozen/core-player-culture-prefix/7c563c56970ee8fcdfe6c0761d091a5361f909c6f95ce1d8415dc62cb2a2b8f0-69ce855cac97 --output target/linux/core-player-culture-prefix.json
```

A source rebuild requires the exact committed profile source snapshot. The target gate remains Linux Core `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`; 681 combined method fingerprints are checked. Source-only metadata/settled-Continent insertions relocate the complete downstream cursor without changing untouched tail bytes.

The full raw Save→Load difference remains **89,198 bytes**, unmasked. Complete Player/Unit rosters, native load fixups, logical gameplay state, all gameplay RNG, external policy boundaries and action/turn execution remain required work. Pure UI desktop admission is not a simulator prerequisite. See [NameManager](linux-player-name-manager.md), [Culture](linux-player-culture.md) and the earlier [Districts-stage profile](linux-core-player-districts-prefix.md) for exact component coverage and historical boundaries.

The integrated workspace checks pass: 191 c6-core tests, the remaining workspace tests, strict all-targets Clippy and the C ABI smoke check. Independent source acceptance is recorded in [ee62de4](linux-player-culture-prefix-review.md). The profile directory includes the candidate hash and source-manifest digest prefix because a corrected driver comment produced the same executable hash. A later unrelated world.rs edit did not alter this candidate: its exact tested source was recovered by digest from the earlier closed archive, with zero unmatched files.
