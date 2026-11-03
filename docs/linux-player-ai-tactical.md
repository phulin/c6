# Linux Player AI.Tactical saved wire

The bounded codec covers the complete **current Tactical v2 saved record**, using the existing exact shared `player_ai_homeland::MoveList`. It does not reconstruct a live Tactical object or execute native Read, decisions or game RNG. Full Player/world state, actions and turns remain unsupported; the recorded **89,198-byte raw Save→Load difference remains unmasked**. No planning-only exclusion follows from this codec.

The target is Linux XP2 `libGameCore_XP2.so`, SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. The [audit](../tools/linux/player_ai_tactical_audit.py) pins complete method extents, direct/PLT bindings, shared MoveList virtual bindings and Tactical DWARF. The [report](../evidence/linux/player-ai-tactical.json) records actual source framing and immutable exported Rust candidate identities. Discovery attribution follows [reference.md](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; target symbols, DWARF and bytes establish this contract.

The little-endian wire is:

| Field | Encoding |
|---|---|
| Version | `u32(2)` |
| Player identity | `i32` |
| Control instance ID | `u32` |
| Prioritized moves | Shared `MoveList v1`: `u32(1)`, counted `i32` base moves, counted `Move v2` records |
| Zombie UnitAI type | Raw `i32` ordinal |

Each Move v2 is `u32(2)`, `u32 AllowedMove definition hash`, `i32 priority`. Counts are u32. List elements consume shared item credit; scalar/version/count words do not. Fixed Tactical overhead is 16 bytes plus the exact shared MoveList encoding. No bool, implicit padding, priority, current-turn unit list, used-unit list or TacticalUnit vector is written. Legacy versions are rejected explicitly.

Writer `0x2195480` (110 bytes) serializes Player identity from pointer `+0xa0`, wrapped control ID `+0xe8`, wrapped prioritized moves `+0xa8`, then `m_zombieType` at `+0x1b0`. UnitAI writer `0x2d4fc30` (33 bytes) copies the 32-bit value and invokes the signed-int stream operator; reader `0x2d4fc60` (38 bytes) reads signed int and copies it back. There is no definition lookup or hash translation in those enum operators. This differs from the nested Move AllowedMove field.

Native Read `0x2195510` (771 bytes) resolves Globals.GetPlayer, reads control ID and conditionally reconstructs Control callbacks, and subscribes turn, turn-end, deactivation and activation callbacks before reading MoveList. `ReadVersioned<UnitAITypes>` at `0x21a3cb0` (28 bytes) reads the trailing ordinal for versions at least 2 and otherwise installs -1. Native Read then **overwrites that field** using UnitAiTypeDefinitions.FindIndex of `0x1b7a9571`, and calls InitArchive (`0x2195e90`, 175 bytes). These are native lifecycle effects, not the saved-wire grammar or permission to run native policy. The codec preserves the saved ordinal exactly, including noncanonical signed values.

The two SHA-gated original Core streams each contain 43 actual Player records, all WasEverAlive=true. There is no Research or Tactical hook. The audit begins at each same-Player Military hook end, decodes the exact current Research grammar, then decodes Tactical to the same-Player Targets hook start and requires Targets v4 plus matching Player identity. This uses source grammar to determine the Tactical extent; fixture cuts never enter the production parser. All 86 records are projected independently into typed values and repacked exactly. The Rust candidate compares the typed projection digest, all bytes, identity, exact prefix consumption and item totals, exact and one-short byte/item budgets, strict trailing rejection, and structural/interior truncations. Small source carriers cover every truncation plus hostile counts and versions.

Reproduce without original-code execution or private writes:

```sh
python3 tools/linux/player_ai_tactical_audit.py --source-commit COMMITTED_EXPORT --output target/linux/player-ai-tactical.json
```

The candidate requires the committed module export and committed driver. It builds an unchanged Git archive, verifies every archived regular source against Git before and after builds, runs actual focused Cargo tests and strict all-target Clippy, and records candidate executable identity. Existing original Save outputs provide structural validation; no new original Read oracle or gameplay comparison is claimed. The next unsupported Player suffix component is **AI.Targets v4**.
