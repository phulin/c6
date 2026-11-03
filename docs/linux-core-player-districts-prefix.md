# Direct Core prefix through first Player Districts

The current `core_stream::ThroughFirstPlayer` API reads from byte zero through the first Player's complete Districts envelope. It composes the earlier Map, Religion, Player header, Formations, Goody_Hut, Bonuses, Congress and Cities codecs with complete current Diplomacy50/Blackboard13 and Districts8/District15 codecs. The next unparsed schema is **Player.NameManager**. No Player entry is complete, and no complete Player/Unit roster, executable world or gameplay parity is claimed.

The [driver](../crates/c6-differential/src/bin/c6-core-player-districts-prefix-candidate.rs), [audit](../tools/linux/core_player_districts_prefix_audit.py) and [report](../evidence/linux/core-player-districts-prefix.json) define this separate profile. The earlier Cities-stop profile and candidate `2578ba02…` remain immutable historical evidence. The new candidate is `5edcd6dfd2cdf669600d06b571a6542cee440b610902f57ff46d899d3ff196b0`; its exact executable, report and 56 source files are frozen locally under `target/frozen/core-player-districts-prefix/<candidate-sha256>/`.

```rust
use c6_core::{core_stream::ThroughFirstPlayer, map, unit};

let prefix = ThroughFirstPlayer::decode(
    &raw_core,
    map::Limits::default(),
    unit::Limits::default(),
)?;
let first = prefix.player_prefix.first_player.as_ref();
let tail = prefix.tail; // borrowed; the first Player is still unfinished
```

Input is the decompressed raw Core stream, not a host save container. The library derives every cursor structurally; no Python offsets, native traces, known-stream digest or byte-pattern searches are parser inputs. Unsupported unframed versions/providers remain errors. `consumed` is absolute, `items_used` spans the initial prefix and the later components, and the tail borrows the input without copying or semantic classification. Shared byte/item credits are passed to each child. Map allocation/depth limits apply through Map; later components retain their documented aggregate-item and checked per-vector allocation policy.

## Original-wire comparison

Both immutable 80,657,462-byte original streams produce:

| Observation | Result |
|---|---:|
| Map end | 4,695,864 |
| Post-Map consumed bytes | 292,438 |
| Post-Map dynamic items | 43,794 |
| Absolute consumed position | 4,988,302 |
| Aggregate items | 2,001,890 |
| First Player Formations / Cities / Districts | 35 / 11 / 72 |
| Unfinished Player entries | 43 |
| Borrowed unparsed tail bytes | 75,669,160 |

The entire typed post-Map region re-encodes byte-identically, SHA-256 `f007a724262ac3dc0665acc7a27e94fe69922d18350c7b5077b858eff3edb684`. The audit independently checks Religion and header values, ordered container slots/free metadata, all City/District record hashes and basic identities, complete Cities/Districts tails and the native outer component boundaries. The complete component audits additionally check all original Diplomacy/Blackboard/Tracker and District records across both streams; the same fixture context is not presented as independent gameplay branch coverage.

The historical owner traces have **no NameManager entry hook**. They prove the Districts end; exact Player writer `0x2b4ee00` establishes Districts→NameManager→Culture ordering. The report explicitly distinguishes that source evidence from an observed callback. Native owner cuts are independent validation inputs only and never reach the Rust parser. No native `Read`, initialization callback or original simulation code executes here.

The audit gates exact Linux Core `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, matching host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4` and 498 combined method fingerprints. New component overload/call/provider contracts have separate [Diplomacy](linux-player-diplomacy.md), [Blackboard](linux-player-diplomacy-blackboard.md) and [District](linux-player-districts.md) audits. The District free-link allocation guard was corrected before integration and independently checked against adversarial byte budgets.

Source-only cases add 20 bytes of framed Context metadata or one four-byte settled-Continent entry. The decoder relocates subsequent boundaries, roundtrips the altered typed region and preserves the untouched tail. These are grammar checks, not new native gameplay fixtures. Exact/one-short limits, truncations, model inconsistencies and invalid versions/bools have focused tests and independent composition checks.

Replay this immutable local profile with (a source rebuild instead requires its exact committed source snapshot):

```sh
python3 tools/linux/core_player_districts_prefix_audit.py --frozen-profile target/frozen/core-player-districts-prefix/5edcd6dfd2cdf669600d06b571a6542cee440b610902f57ff46d899d3ff196b0 --output target/linux/core-player-districts-prefix.json
cargo test -p c6-core core_player
cargo test -p c6-core core_stream
```

Later schema expansions can replay the immutable candidate using the audit's `--frozen-profile target/frozen/core-player-districts-prefix/<candidate-sha256>` option. Frozen source and executable hashes must match before replay. Rebuilding a different source profile does not retain this candidate identity.

Independent codec review [835f3b6](linux-player-diplomacy-districts-review.md) and composed-prefix review `bb69fdc` accepted the committed source behavior. The recorded full raw Save→Load difference remains **89,198 bytes**, unmasked. Metadata compatibility, runtime player-slot validity, definition remapping, native Read padding/fixups, complete logical gameplay state and all gameplay RNG remain separate requirements. Actual District owner comes from the relevant Plot owner; the enclosing Player/table ID does not substitute for that value. UI desktop admission is not a prerequisite for this gameplay-only library boundary.
