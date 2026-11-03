# Linux AI Economic saved wire

The bounded [Rust codec](../crates/c6-core/src/player_ai_economic.rs) implements the complete current saved grammar of Linux XP2 `AI::Economic` v4, including `AI::Trade::Player` v3 and `AI::Control::Handle` v1. This is state transport; full Player/world import, gameplay actions, exact gameplay RNG and turns remain unsupported. No planning-only exclusion is established. The recorded **89,198-byte raw Save→Load difference** remains unclassified and unmasked.

The read-only [audit](../tools/linux/player_ai_economic_audit.py) gates ELF SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, concrete serializer/helper extents and DWARF layouts. Supplied Linux bytes are authoritative; discovery names follow the [attributed reference](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`. No original code executes and no proprietary artifacts are written.

Economic writer `0x20a3110` and reader `0x20a3260` establish this order. All words are little endian; Q8 values preserve the full signed raw `i32`, without scaling or rounding.

| Field | Wire |
|---|---|
| Version, Player identity | `u32(4)`, `i32` |
| Current balance, city income, religious income, diplomatic income, percentage bonus income, city expenses | Six raw Q8 `i32` words |
| Turns with negative income, total saving weight | Two `i32` words |
| Saving weights | `u32` count, ordered `i32` values |
| Savings | `u32` count, ordered raw Q8 `i32` values |
| Yield priority boost | `u32` count, ordered `(Yield hash u32, raw Q8 i32)` entries |
| Managed contracts | `u32` count, ordered Control.Handle records |
| Trade manager | AI.Trade.Player record |

Yield array writer `0x1ff74c0` traverses definition indices in ascending order. The wire model preserves saved order, duplicate hashes and every sentinel bit; it does not resolve or remap hashes into live definitions. Each Control.Handle is `u32(1), i32(player), u32(id)` (writer `0x2018250`, reader `0x20182a0`). Each AI.Trade.Player is exactly `u32(3), i32(player), u32(control_id), u32(best_tech_driven_road_hash), i32(turns_since_railroad_tree)` (writer `0x21b4770`, reader `0x21b47e0`). The Route binding (`0x2d4f5e0` / `0x2d4f610`) proves that the road is a definition hash, not its live signed enum index. AI.Trade.Player is independent of Player.Trade v35. Neither of these records nor Economic emits a bool object byte.

Economic Read resolves the global Player after the six Q8 fields, conditionally initializes signals for non-barbarians, and initializes its archive. It sizes the live Yield vector from current definitions and initializes it to zero before reading/remapping hashes. The native v2/v3 compatibility gates introduce income fields and v4 introduces Yield boosts; this codec accepts only current v4. Nested Trade Read resolves its own Player, reads v2 Route/v3 railroad-counter fields and sets its initialized flag. Control.Handle loading reads its saved Player identity and ID without resolving a live instance; obtaining usable contracts remains separate. Archive ownership, signals, global mapping, defaults for unmatched definitions and native initialization are not reproduced by parsing bytes. No claim of no transitive RNG use follows from this audit.

Existing owner traces contain **no direct Economic or AI.Trade.Player hook**. Validation uses each Diplomatic owner end and matching Governors owner start as contextual interval endpoints, checks enclosing Player v47 identity and decodes/reconstructs the complete independently derived grammar. These endpoints never enter the production parser. Across both SHA-gated canonical streams all 86 intervals parse exactly (43 per stream), with 124–612 bytes and 6–52 aggregate dynamic items. This compares existing native Save outputs, not native Read execution or initialized gameplay acceptance.

The immutable committed-source candidate checks all 86 typed projections and byte roundtrips, exact and one-short aggregate byte/item caps, strict trailing refusal, and prefix consumption with a tail. Native truncation cuts cover structural boundaries plus first/last repeated row scalar boundaries and one interior byte; the small hostile carrier checks every truncation, unsupported top/nested versions, maximum counts, signed extrema, unsigned extrema, duplicate hashes and sentinel preservation. Owned vectors use checked bounds and fallible allocation. Encoding preflights the entire byte/item requirement before allocating its output.

Reproduce after committing the exported module and unchanged audit driver:

```sh
python3 tools/linux/player_ai_economic_audit.py --source-commit COMMIT --output target/linux/player-ai-economic.json
```

The [report](../evidence/linux/player-ai-economic.json) records the immutable archive, every archived regular source identity checked before/after builds, actual Cargo module tests, strict Clippy and candidate binary identities. The next Player dependency is **AI::Governors v3**, separate from Player.Governors.
