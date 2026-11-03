# Current Linux AI.Targets saved wire

The bounded [Targets codec](../crates/c6-core/src/player_ai_targets.rs) covers the complete current saved wire of AI.Targets v4 and its transitive knowledge records. It follows Tactical and ends immediately before BehaviorTree.Manager v1 in current Player v47. This is saved state transport, not a reconstructed native Targets object, gameplay execution, or native Read lifecycle. Knowledge, diplomacy, gameplay state and RNG remain in scope regardless of AI ownership. Complete Player/world import, actions and turns remain unsupported; the recorded **89,198-byte raw Save→Load difference stays unmasked**.

The [read-only audit](../tools/linux/player_ai_targets_audit.py) gates Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, 200 complete concrete serializer method extents, 705 direct PLT call/jump bindings and the selected DWARF layouts. The [report](../evidence/linux/player-ai-targets.json) records those identities. No original game code executes; no Ghidra project, private artifact, native runner, host configuration, rules database or Steam installation changes. Discovery attribution remains the [cru121 reference](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; supplied Linux symbols, DWARF and bytes establish this contract.

Targets writer `0x21a79c0` emits, in order: version 4; signed Player ID; counted TargetType definition-hash/raw-bool rows (`m_desiredTargets`, live `+0xb8`); raw bool `m_newWarStarted` (`+0xf0` wrapped value `+0x100`); CivKnowledge (`+0x270` wrapped value `+0x280`); counted PlotCoords `m_operationStartPoints` (`+0x9a0`); signed `m_releaseTurn` (`+0xd8`); counted unsigned-key ExplorationInfo rows for **area visibility before map visibility** (`+0x210`, then `+0x1e0`); TargetList (`+0x240`). This order differs from live member order. No opaque range, owner-cut skip, guessed default, native priority field or callback pointer is serialized by this codec.

All scalar fields are little endian. Version/count/hash words are u32; ordinary enums and Q8 fixed-point raw values are i32, unless the serializer explicitly converts a definition index to a hash. PlotCoord has two i32 values and no version. ComponentID/CityID wire contains i16 owner, i16 component type and u32 ID, without a version; CityKnowledge separately serializes its city owner as i32 plus u32 ID. Exploration maps retain unsigned keys, distinct from definition hashes. Every serialized traversal order and raw value is retained; decoding neither sorts/deduplicates containers nor resolves definitions to current indexes.

| Record | Current version | Ordered saved fields |
|---|---:|---|
| CivKnowledge | 7 | owner; cities; barbarians; PlayerKnowledge; Unit-indexed total-hostile-army counts; WarKnowledge; counted **64** ArmyKnowledge records; TradeKnowledge; Yield-indexed Q8 averages; Yield-indexed Q8 errors |
| CityKnowledge | 7 | i32 owner, u32 city ID; observation age; population; location; region ID; three bool bytes (original-capital, capital, occupied); Religion hash; current loyalty Q8; loyalty change Q8; next civ identity; DistrictDefenses vector; Yield-indexed expected yields; Building-indexed owned bools; Building-indexed in-progress bools; District-indexed counts; raw GreatWorkListIndex vector; Resource-indexed counts |
| ArmyKnowledge | 4 | raw presence bool; if nonzero, Unit-indexed vectors of UnitInfo |
| UnitInfo | 2 | unit ID; known damage; raw MilitaryFormationTypes; location; area ID; region ID; observation age |
| DistrictDefenses | 2 | ComponentID; District hash; location; garrison defense Q8; outer defense Q8; attack strength Q8 |
| BarbarianKnowledge | 1 | camp location; observation age; guard Unit hash |
| PlayerKnowledge | 1 | Player ID; victory progress Q8 |
| WarKnowledge | 1 | opponent; goal score Q8; current tally Q8; active bool byte |
| TradeKnowledge | 1 | owner; from CityID; to CityID; origin Q8 yield vector; destination Q8 yield vector; raw destination best YieldTypes **before origin**; total Q8 value |
| ExplorationInfo | 1 | number of cells; cells visible **before cells revealed** |
| TargetList | 1 | counted AiTarget vector |
| AiTarget | 3 | raw AiTargetType; location; ComponentID; target range; target priority; three bool bytes (refresh location, refresh priority, currently visible) |

`CivKnowledge::Write`/`Read` are `0x1ffd320`/`0x1ffd430`; CityKnowledge methods are `0x1ffc7e0`/`0x1ffc990`; ArmyKnowledge operators are `0x1ffceb0`/`0x1ffcf20`; AiTarget operators are `0x21a7720`/`0x21a77b0`; TargetList operators are `0x21a7950`/`0x21a7990`. The report pins all current nested serializer paths, scalar widths, fixed array framing, hash conversions, bool serialization and raw enum wrappers. It separately retains versioned reader branches as evidence; unsupported historical versions are rejected by the codec rather than emulated.

`BoolByte(u8)` preserves saved bool storage without claiming a canonical truth value. The bool scalar writer delegates to a one-byte object write; it does not normalize to 0/1. Wrapped fields resolve through their actual FAutoVariable value storage. ArmyKnowledge's presence branch uses zero versus nonzero, retaining the original byte; inconsistent caller-constructed `Conditional` shape is rejected. The native current ArmyKnowledge writer computes a canonical presence flag from its vector size, but that does not justify normalizing any other bool storage. All 256 bytes are exercised in AiTarget bool carriers; noncanonical conditional flags are exercised separately.

Targets reader `0x21a7a80` resolves the Player, selects prioritization callbacks according to IsBarbarian, reads versioned fields, initializes signals/archive and sets initialized. CivKnowledge readers perform definition lookups, vector sizing and live-state initialization. This codec performs none of those effects, no decisions, and no RNG draws. Neither absence of direct RNG calls nor the namespace proves a planning-only exclusion or RNG isolation.

Validation uses both SHA-gated canonical Core streams and their SHA-gated comparison-only owner traces. All 86 actual Targets hook extents independently parse/repack exactly, agree with enclosing Player v47 identity, and end at the next BehaviorTree.Manager identity. Every nested actual TradeKnowledge and AiTarget hook extent matches the typed grammar. The actual hook inventory is reported, with the real Targets parent context; unhooked nested types are validated by grammar rather than invented trace coverage.

The committed-source candidate uses unchanged exported commit `ae2eb69` through `git archive`, including the parent-owned public crate export. It verifies every archived regular source against immutable Git content before and after Cargo builds, runs focused actual crate tests and strict all-target Clippy, and links its typed projection harness against that crate. The 86 actual records total 23,242,766 bytes (49,221–663,978 bytes each), with exact aggregate byte/item caps, one-short caps, strict trailing rejection and prefix consumption. Checks include 41,670 bounded structural/interior truncation cuts and 4,816 hostile count/version sites. For large repeated records the selected cuts cover container/count/scalar boundaries and first interior bytes of the first/last repeated rows; they do not exhaust every byte of every middle row. Small synthetic transitive carriers exhaust truncations; large synthetic top-level carriers use bounded edge cuts. These are current saved-output codec comparisons, not a new original Read oracle or complete subsystem/gameplay parity.

Reproduce the pinned report:

```sh
python3 tools/linux/player_ai_targets_audit.py --source-commit ae2eb69 --output target/linux/player-ai-targets.json
```
