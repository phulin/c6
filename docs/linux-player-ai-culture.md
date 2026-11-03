# Linux AI.CultureAi saved wire

The bounded [codec](../crates/c6-core/src/player_ai_culture.rs) preserves the complete current v15 **AI.CultureAi saved record**, distinct from gameplay Player.Culture. It does not execute policy, rebuild native objects, advance gameplay or classify these fields as planning-only. Full Player/world import, complete gameplay state, actions and turns remain unsupported; the recorded **89,198-byte raw Save→Load difference remains unmasked**.

The read-only [audit](../tools/linux/player_ai_culture_audit.py) gates Linux XP2 module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and complete writer/reader/helper method extents. Original target symbols/DWARF/bytes establish the schema; [reference attribution](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, applies to discovery names only. No original code executes and no private artifacts are written.

Writer `0x2028c60` (444 bytes) and reader `0x2028e20` (1227 bytes) establish the following order. All words are little endian. A vector is a u32 count followed by its ordered elements, with no container version/capacity/padding. An indexed array is a vector of `{u32 definition_hash, typed value}`; hashes, duplicates and saved order are preserved without definition remapping. Q8 fields retain signed i32 raw storage. Player IDs and option/era/turn/plot values are raw signed i32 ordinals or integers. Native bool serializers forward one object byte; `BoolByte(u8)` preserves all representations without normalization or a truth-value claim.

| Order | Saved fields |
|---|---|
| Header | u32 version15, i32 player ID |
| Control | vector of Handle v1: version, i32 player ID, u32 control ID |
| Boost | BoostManager v1: version, i32 player ID, u32 allowed active boosts, vector of `{u32 BehaviorTreeControlId, i32 BoostUniquenessHash}` |
| Flag | doneOffTurn bool byte |
| Preferences | indexed Q8 PseudoYield congress preferences; indexed Q8 PseudoYield preferences; indexed Q8 GovernmentSlot best policy values |
| Analysis | indexed CivicAnalysis; indexed PolicyAnalysis; indexed ResolutionAnalysis |
| Government | indexed Q8 Government values |
| Rock bands | vector of RockBandAnalysis |
| Great works | GreatWorks v3: version, i32 player ID, Q8 greatWorkValues vector, Q8 workingGreatWorkValues vector |
| Tail | i32 nationalParkSpots vector, i32 numNaturalists, mustChangePolicies bool byte |

CivicAnalysis v2 (`0x20280d0`) writes player ID, Civic hash, next-needed Civic hash, four bool bytes (owned/favored/boost/repeatable), solo/full Q8 values and i32 turnsToUnlock: 32 bytes. Its live unlocked-government/policy lists are not saved. PolicyAnalysis v4 (`0x2028680`) writes player ID, Policy and GovernmentSlot hashes, dark/golden-age bool bytes, min/max era chronology and Q8 value: 30 bytes. ResolutionAnalysis v2 (`0x2028950`) writes player ID, Resolution hash, Q8 overall/option-one/option-two values and i32 best option: 28 bytes. RockBandAnalysis v1 (`0x2028b50`) writes player ID, base Q8, PlotCoord two i32 coordinates, District and Improvement hashes, and nationalPark bool byte: 29 bytes. Its live bestUnit is not saved. Versions contribute four bytes to each extent.

Boost uniqueness hashes are raw signed integers, not Civic/Technology definition hashes. GreatWorks writer `0x2053dc0` saves no requests or doneOffTurn flag. Control handles save no resolved contract pointer. Native CultureAi Read resolves Player identity, edits archive wrappers, calls InitArchive/InitializeLists, initializes GreatWorks, and resets four analyzers after reading. GreatWorks Read sets doneOffTurn false and clears requests. BoostManager Read appends saved managed boosts to its existing vector and calls Initialize to rebuild support functions; the Culture archive load wrapper does not reset it first. A standalone saved-wire value cannot reproduce these live-state effects. Native AI policy choices, gameplay-affecting state and RNG remain in scope and unclassified.

Resource limits charge actual consumed wire bytes and every vector element cumulatively, including hash-array entries and nested vectors. Each decoder count preflights minimum available wire, checked typed allocation size and aggregate item capacity before fallible reservation. Encoding validates the complete traversal before one fallible output reservation, then writes into that reserved capacity. There is no aggregate heap-byte-cap claim; `max_items` bounds typed dynamic element counts.

The audit checks actual CultureAi owner hooks (43 in each canonical Core stream), enclosing Player v47 identity, nested Boost/Civic/Policy/Resolution/RockBand hook counts and the next Diplomatic boundary. Those offsets are comparison evidence, never parser inputs. Python independently reconstructs the typed fields and emits a semantic scalar projection compared with the committed exported Rust candidate; candidate checks roundtrips, every truncation, exact and one-short aggregate byte/item budgets, strict trailing refusal and shared-tail prefix consumption. Source-only tests cover noncanonical bool bytes, signed/Q8 extremes, hash sentinels, managed boosts/contracts, hostile counts and unsupported nested versions. No new original Read oracle or gameplay acceptance is claimed.

Reproduce with an exported source commit containing the actual driver and module:

```sh
python3 tools/linux/player_ai_culture_audit.py --source-commit COMMIT --output target/linux/player-ai-culture.json
```

The candidate builds an unchanged Git archive, verifies all archived regular source files against the Git pin before/after, runs actual targeted Cargo tests and strict all-target Clippy, and records archive/source/harness/executable identities. Missing public export is a build failure; the audit never injects one. The next unimplemented Player record is **AI.Diplomatic v18**.
