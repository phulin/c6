# Linux AI.Homeland saved wire

The current Linux XP2 Homeland record and its complete transitive MoveList wire are typed in [player_ai_homeland.rs](../crates/c6-core/src/player_ai_homeland.rs). This covers saved bytes only. Native Player lookup, Control callback restoration, subscriptions, live unsaved fields, definition remapping, policy execution, gameplay RNG, full Player/world state, actions and turns remain unsupported. No planning-only exclusion is established; the recorded **89,198-byte raw Save→Load difference** remains unmasked.

The read-only [audit](../tools/linux/player_ai_homeland_audit.py) gates the supplied Linux `libGameCore_XP2.so` SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, complete concrete method extents, primitive/sequence bindings, virtual load/save slots and DWARF. [Reference attribution](reference.md), discovery revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, applies; target symbols/DWARF and bytes establish this grammar. No original code executes and no private artifact is written.

All fields are little endian, with no padding. Counts are `u32`, and saved order, duplicates, arbitrary signed values and hash sentinels are preserved.

| Record | Current grammar | Writer / reader |
|---|---|---|
| Homeland | `u32(1), i32 player, u32 control_instance_id, i32 additional_priority, MoveList` | `0x2134880` / `0x2134910` |
| MoveList | `u32(1), u32 base_count, i32 base_moves[base_count], u32 move_count, Move[move_count]` | `0x215a450` / `0x215a4a0` |
| Move | `u32(2), u32 allowed_move_hash, i32 priority` | `0x215a380` / `0x215a3f0` |

The Homeland priority is specifically `m_additionalPriority.m_value` at live offset `+0xf8`, not the separate `m_priority` at `+0xe0`. ControlPriority enum helpers `0x213d450` / `0x213d3b0` serialize signed ordinals. Move v2 instead uses `WriteIndexToHash<AllowedMove>` / `ReadHashToIndex<AllowedMove>` (`0x215a8a0` / `0x215a8f0`), saving a definition hash; its legacy v1 ordinal branch is refused by this current-version codec. Base moves are signed integers, as shown by DWARF and their actual vector binding. There are no bool fields to normalize.

The reusable public MoveList APIs expose strict and prefix decode, consumed item counts, encode and encoded item counts with `unit::Limits`. Homeland uses the same typed grammar internally. Each base move and each Move contributes one dynamic item; version words, counts and scalar fields do not. Counts, aggregate byte/item budgets and owned allocation sizes are checked; allocation is fallible and encoding validates before reserving the exact output. Unsupported versions, truncation, strict trailing data and one-short limits fail explicitly.

Homeland's MoveList FAutoVariable load/save bodies `0x213ee70` / `0x213eee0` and Tactical's `0x21a4230` / `0x21a42a0` add `+0x10` to the wrapped object and tail-call the **same** MoveList reader/writer PLT bindings. The audit checks their concrete virtual slots and PLT target resolutions, establishing the reusable binding without claiming Tactical's enclosing record is implemented.

Native Read resolves the Player, restores the Control.Instance callback when the saved ID is valid, and subscribes Homeland callbacks to Player turn activation, turn and turn-end signals. It finishes with `InitArchive` (`0x21351c0`), which sets the archive identity using component type `0x36` for participants or the invalid identity otherwise. These lifecycle effects are outside wire transport. Neither saved-field retention nor an absence of direct RNG calls proves transitive RNG isolation or permits running native policy choices automatically. All player decisions must yield to external policy; exact upkeep and all gameplay-affecting state/RNG remain required.

The two canonical Core streams and owner traces are SHA-gated. Neither trace hooks GrandStrategic or Homeland. For each Player, the audit decodes the complete GrandStrategic grammar from Governors.end, then parses Homeland's own grammar. It independently requires the derived end to equal the same Player's Military hook start and validates Military v14/player identity. These source intervals validate the codec; production parsing uses no fixture offsets. All 43 actual Players in each stream have WasEverAlive=true. Every one of the 86 Homeland records has 36 base moves and 36 Move v2 rows: 604 bytes and 72 dynamic items. Actual fixtures therefore cover nonempty current records but do not establish legacy, empty-vector native output, or native Read behavior.

Reproduce the target/DWARF/actual-output structural audit without game execution:

```sh
python3 tools/linux/player_ai_homeland_audit.py --output target/linux/player-ai-homeland.json
```

Add `--source-commit COMMIT` for the committed, publicly exported Rust codec comparison. The candidate builds an unchanged immutable Git archive, verifies every archived regular source against Git before and after builds, runs focused Cargo tests and strict all-target Clippy, then compares an independent full typed projection and exact re-encoding of all 86 records. It checks actual scalar/container boundaries and interior cuts, strict trailing refusal, prefix consumption and exact/one-short byte and item limits. The small synthetic carrier checks every truncation, empty vectors, hostile counts and every supported record version. A missing public export fails the candidate build; source is never injected into the archive. [Evidence](../evidence/linux/player-ai-homeland.json) records the candidate commit and executable identity when that comparison is complete. This is an offline comparison against original Save outputs, not a new native Read oracle or state/action/turn acceptance.

The next unimplemented Player component after Homeland is **AI.Military v14**, writer `0x21453f0`, reader `0x2145620`.
