# Linux GlobalAi current wire

The `game_global_ai` module transports the complete current `GameCore::AI::GlobalAi` record of the pinned Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. It does not execute native Read, initialize AI, remap definitions, subscribe signals, make decisions, or advance gameplay.

GlobalAi Write at `0x20f41d0` (222 bytes) emits version 4. Read at `0x20f42d0` (820 bytes) also supports historical branches; this codec explicitly rejects those versions. The outer free wrappers are `0x20f41b0` and `0x20f42b0`. The Game writer calls Player.Manager then GlobalAi at `0x2d7a866` and `0x2d7a879`; the Game reader uses `0x2d7ab6f` and `0x2d7ab9a`.

All integers are little endian. Hash arrays retain ordered `(u32 definition hash, value)` pairs, including duplicate/unknown hashes. They are not remapped into a rules registry. Raw bool carriers are `u8`; native bool bindings transfer one byte, and Religious gates its body on nonzero. Mayhem retains all native IEEE-f32 bits, including NaNs. Q8 values retain signed raw i32 words without numeric conversion. Offset-based field names identify concrete wire bindings where semantic names have not been established.

| Record | Current wire order |
| --- | --- |
| GlobalAi v4 | wrapped u32; MayhemTracker; ControlFactory; EspionageManager; ReligiousManager; Q8; counted i32 vector; two counted Yield-hash/Q8 arrays |
| MayhemTracker v1 | native float bits |
| ControlFactory v1 | u32 |
| EspionageManager v1 | counted AI::Espionage records |
| AI::Espionage v4 | player ordinal i32; priority i32; counted city analyses; raw bool; counted CityID vector; three counted i32 vectors |
| EspionageCityAnalysis v3 | CityID; count **11** followed by eleven Q8 words; i32 |
| ReligiousManager v2 | counted AI::Religious records |
| AI::Religious v6 | raw initialized bool; conditional body |
| Religious conditional body | player ordinal; Religion hash; raw bool; counted Belief-hash/BeliefAnalysis pairs; counted ControlHandle vector; ControlHandle; raw bool; counted CityReligionAnalysis vector; counted Yield-hash/Q8 array |
| BeliefAnalysis v1 | player ordinal; Belief hash; BeliefClass raw ordinal; raw bool; Q8 |
| CityReligionAnalysis v4 | CityID; two PlotCoords; three Q8 words (native object offsets 0x18, 0x24, 0x1c); raw bool; counted u32 vector |
| ControlHandle v1 | player ordinal; u32 ID |
| CityID | owner i16; component type i16; ID u32 |
| PlotCoord | x i32; y i32 |

AI::Espionage is a different concrete class from Player.Espionage v17. No Player espionage codec is reused. The fixed act array is the current writer's eleven entries; noncurrent lengths fail explicitly. The transport does not reproduce native Read's resizing of the final espionage vector to eleven entries, signal initialization, player lookups, belief initializer effects, analyzer initialization, or archive setup. Those effects require live state and evidence beyond this transport boundary. No AI-namespaced data is classified as planning-only or discarded.

`decode`, `decode_prefix`, `decode_prefix_with_items`, `encode`, and `encode_with_items` share `unit::Limits`. Byte caps charge actual consumed/emitted wire bytes. Item caps aggregate every counted vector/hash array and the eleven-element act array across all nesting. Allocation sizes use checked multiplication/addition and fallible reservation; count checks enforce remaining minimum wire and shared item budgets before allocation. Rust struct padding is not charged as wire bytes. Inconsistent Religious flag/body representations fail explicitly.

The read-only `tools/linux/game_global_ai_audit.py` gates the original ELF and complete relevant function bytes, independently reconstructs every typed field, compares all direct native child extents, and tests an unchanged Git archive using the committed exported crate and actual Cargo tests/strict Clippy. Missing exports fail; the driver never injects source. Small carriers exercise every raw bool value and every byte cut, native float bit patterns, hostile counts/versions and inconsistent conditional bodies. Native records use bounded cuts and hostile nested version/count sites, exact aggregate caps, one-short caps, prefix extents and strict trailing-byte rejection.

Both canonical streams contain the same 368,461-byte GlobalAi record at comparison extent `70566134..70934595`, with 47,501 counted items and 43 Espionage plus 43 Religious direct children. Every Religious record is initialized in those fixtures, so synthetic tests cover the absent branch. Their wire SHA-256 is `d70893e3576acd575d02df74e7ddae85901e9f98311d24a2719f06c7744ed05b`. Trace offsets are comparison evidence only and are never production parser inputs. The next Core owner is Barbarian.Manager at `70934595`.

This adds a bounded current wire component, not full Player/world import, gameplay state/RNG, actions, or turns. External policies must supply every player decision at future gameplay decision boundaries. The recorded raw Save→Load difference remains **89,198 bytes**, unmasked and unclassified. Original code execution, live initialized-game acceptance, and full gameplay equivalence remain unsupported by this audit.
