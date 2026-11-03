# Linux AI Research saved wire

The bounded current `player_ai_research::Research` codec retains the complete
saved Research v4 wire for Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
It implements no decisions, gameplay effects, native load initialization or turns.
The [audit](../tools/linux/player_ai_research_audit.py) reads the original ELF,
symbols and matching DWARF, and compares existing original save outputs without
executing original code. Discovery attribution is the pinned reference revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`; see [reference.md](reference.md).

Writer `0x2183d20` and reader `0x2183e40` establish this ordered grammar. All words
are little endian, four bytes; counts are unsigned words. Hashes retain their raw
bits, order, duplicates and sentinels. No definitions are resolved by the codec.

| Field | Wire |
| --- | --- |
| Version; Player identity | `u32(4)`, `i32` |
| Research target | Technology definition hash `u32` |
| Turns to research units | count, repeated Unit hash `u32` and `i32` |
| Favored technologies | count, repeated Technology hash `u32` and raw bool byte |
| Tree analysis | count, repeated Technology hash `u32` and TechTreeAnalysis |
| Done off turn | raw bool byte |
| Managed contracts | count, repeated ControlHandle |
| Boost manager | shared BoostManager v1 grammar |
| Turns remaining on current research | `i32` |

TechTreeAnalysis v3 writer `0x2183b10` emits version `i32(3)`, Technology hash,
next Technology hash, Player identity `i32`, turns to research `i32`, value and
full value as raw signed Q8 words, then four raw bool bytes in this order:
has technology, repeatable, has boost, leaf. Its exact saved size is 32 bytes.
Native unit/building vectors, pointer and object padding are absent. Its reader
`0x2183be0` resolves definitions and Player identity; historical branches do not
establish supported older codec versions. ControlHandle v1 is version, Player
identity and unsigned control ID (12 bytes).

The concrete [shared BoostManager contract](linux-ai-boost-contract.md) proves
Research and Culture bindings to the same implementation. This codec calls the
public bounded `player_ai_culture::BoostManager` methods, preserving one source
of grammar. Nested consumed bytes and managed pair items charge the enclosing
limits. Every counted row charges one item, including hash-array rows; bytes and
item totals are checked across all nested containers. Encoding sizes first and
uses fallible owned allocations. Native bool storage bytes survive unchanged,
including noncanonical carriers; the codec does not infer truth or normalize.

The original reader resolves Player, calls InitializeLists, marks TechAnalyzer
updated, initializes its archive and marks itself initialized. Nested BoostManager
load appends to existing native records and initializes personality support.
These live effects and unsaved dependencies are distinct from wire retention.
Research::DoTurnActivate (`0x2185a70`) and QueueNextTech (`0x2186110`) include
policy selection; fallback consumes shared Game.Random and mutates Player.Techs.
The codec executes no policy. External policies must supply all player decisions;
no planning-only or RNG exclusion follows from this saved record.

Run `python3 tools/linux/player_ai_research_audit.py --output /tmp/research.json`;
add `--source-commit COMMIT` for an unchanged exported immutable Git archive build,
focused Cargo tests, strict Clippy and typed Rust comparison. The driver requires
the module export to exist in that commit and never injects source into its archive.
It verifies every archived source against Git before and after building.

The two canonical streams contain 86 Research records, 4,134–4,174 bytes each,
with 6,622 nested TechTreeAnalysis records. Existing traces have **no Research
owner hook**. The audit begins at the hooked Military end, consumes the complete
Research grammar, verifies actual contained TechTreeAnalysis/BoostManager hooks,
and checks the following Tactical v2 identity header. Hooks are comparisons,
never production framing. Each stream has all 43 PlayerWasEverAlive records.
Actual typed projection, re-encoding, bounded structural/interior truncation cuts,
exact/short aggregate caps, and strict/prefix behavior are checked. Synthetic small
carriers cover every truncation, noncanonical bools, unknown versions and hostile
counts. These are existing Save-output comparisons, not a native Read oracle.

Tactical follows Research. Complete Player/world import, gameplay state, action
execution and turns remain unsupported. Unknown save bytes remain unclassified
and unmasked; the recorded raw Save→Load difference remains **89,198 bytes**.
