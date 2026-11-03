# Independent Linux Targets / BehaviorTree wire review

The reviewed current saved-wire boundary passes the checks below. This accepts bounded transport of the supplied original Save records; complete gameplay state, initialized World import, native Read reconstruction, action execution, turns and gameplay RNG remain unimplemented/unaccepted. The **89,198-byte raw Save→Load discrepancy remains unclassified and unmasked**. No AI namespace or saved field receives a planning-only exclusion.

The [reproducible review](../evidence/linux/player-ai-targets-behavior-review.py) and [result](../evidence/linux/player-ai-targets-behavior-review.json) use one unchanged source archive, `eb5bbd03bdb9fbf5394a7a45a29240a3ac8b8eb5`, SHA-256 `d0cc616c190858932a90e9d6c4b3d671c086ee687de9a570d241a65994b2e733`. This is the same pin coordinated with the core-oracle-boundary reviewer. Both actual crate exports are present. Every archived regular file is compared with immutable Git content before building and checked again afterward; no export insertion, source recovery, or candidate-source edit occurs. External Rust comparison drivers live outside that archive.

The Linux module is SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; the host is SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. The runner hashes the host but does not execute it. Host ET_EXEC addresses elsewhere in the project are absolute virtual addresses, never base-added RVAs. No original game code, runner, Ghidra project, configuration, or private artifact is changed or executed.

## Evidence and grammar

This review reruns the archived [Targets native contract audit](../tools/linux/player_ai_targets_audit.py) and [BehaviorTree native contract audit](../tools/linux/player_ai_behavior_tree_audit.py). Their reports, fixture pins, field schemas, method identities and reader/writer dispatch contracts are retained in the review JSON. They are reused evidence inputs, not newly invented independent native oracles. Targets gates 200 complete method extents, 705 direct PLT edges, virtual bindings and exact selected DWARF output. BehaviorTree derives 73 writer dispatches and current-v33 reader paths from the target ELF and records 120 relevant methods. The reviewer additionally inspected selected original writer call order against the Rust declarations: Targets `0x21a79c0`, Manager `0x26e4060`, Blackboard `0x26c38f0`, DecisionState `0x26e9eb0`, and NodeDefinition `0x26e9f80`.

Current accepted versions are Targets 4; CivKnowledge/CityKnowledge 7; ArmyKnowledge 4; UnitInfo/DistrictDefenses 2; AiTarget 3; the remaining versioned Targets records 1. ComponentId and PlotCoord are unversioned. BehaviorTree accepts Manager 1, Blackboard 3, StringTable/DataEntry/BlackboardVector/BlackboardData/DecisionState/ControlHandle 1, BehaviorTree 5, NodeDefinition 3, NodeData 33, GameModifierAnalysis 17, and recycled container `0x10001` with block size 16. It requires twelve DataEntry indices. These are current saved-output contracts, without historical-reader emulation.

Field order and widths match the pinned contract tables retained in the JSON and the detailed [Targets](linux-player-ai-targets.md) / [BehaviorTree](linux-player-ai-behavior-tree.md) documents. Specific traps checked in review:

- Targets writes area visibility before map visibility, despite live-member order. TradeKnowledge writes destination best yield before origin. ExplorationInfo writes visible before revealed. Targets' counted army array requires exactly 64 records.
- ComponentId uses signed i16 owner/type and u32 ID; CityKnowledge's separately saved owner is i32. Map keys, IDs, definition hashes, counts, UnitInfo area/region/observation words and ExplorationInfo counters are u32. Plot coordinates, ordinary enum/Q8 words, free-list links and the other signed fields retain their declared i32 storage. Bool storage is one raw byte.
- Blackboard order is StringTable, Units, Techs, Buildings, Ints, Bools, Plots, Strings, Improvements, IDs, FixedPoints, Districts, Civics, then signed job index. Priority rows contain hash/raw-ID, signed weight and signed priority. SharedData contains signed DataTypes and unsigned index, without pointed-to storage. DecisionState's five links are u32 in FirstChild, Jump, LastChild, Parent, StartAt order.
- NodeValue::Word(u32) is a raw four-byte carrier for several native signed/enum and unsigned/hash bindings. Its bits are exact; its Rust variant is not a distinct signed semantic type per native field. The exact C++ serializer bindings stay in NODE_SCHEMAS. Versions sometimes pass through native signed-int overloads; the codec's u32 version tag preserves the current four bytes and rejects other versions.
- Recycled entries and free-index sequences retain order and duplicates. Narrow strings retain byte lengths and non-UTF-8/nul storage; float payloads retain raw bits. No definition remapping, canonical bool conversion, sorting or deduplication occurs.

## Actual comparisons and adverse input

Both SHA-gated canonical Core streams and their unchanged comparison-only owner traces are used. Each contains 43 Targets and 43 BehaviorTree.Manager records directly under actual Player v47 owners. Player IDs agree. Every Targets end is the next Manager header. All original nested TradeKnowledge/AiTarget extents agree with parsed nested extents. Manager ends and enclosing Player ends are also checked through the archived independent PlayerTail grammar; all actual players have WasEverAlive=true. Missing false-branch fixture coverage remains explicit.

| Current saved record | Actual records | Bytes | Aggregate items | Bounded truncations | Hostile count/tag mutations |
|---|---:|---:|---:|---:|---:|
| Targets | 86 | 23,242,766 | 3,172,406 | 41,670 | 4,816 |
| BehaviorTree.Manager | 86 | 3,491,036 | 274,462 | 14,198 | 5,504 |

Every record decodes with its exact aggregate byte/item budget, rejects one-short budgets for both decode and encode, rejects trailing bytes in strict mode, reports the exact prefix extent/items and re-encodes byte-for-byte. Counts share one budget across nested containers. Structural/count/version/scalar boundaries and selected interior cuts are exercised; first/last repeated Targets rows and bounded BehaviorTree samples avoid quadratic exhaustive truncation of large actual records. This is not a claim to have cut every byte of every large record.

The previous Targets audit reduced its complete scalar visitation to an aggregate FNV64 value. This review strengthens that comparison: all **58,333,984 bytes** of ordered signed-i64 scalar/count projection are compared directly, with no digest-only acceptance. Hashes in the JSON identify the resulting artifact after full equality succeeds. Version words are checked separately by grammar and byte roundtrip. BehaviorTree compares its entire **17,989,602-byte** typed Debug projection directly.

The unchanged archived crate runs three Targets tests, four BehaviorTree tests, strict all-target c6-core Clippy and a release build. Small source-only carriers cover all 73 dispatches, all bool bytes, signed extremes, narrow-string bytes, NaN payload bits, all 32 reference-presence combinations, malformed caller shapes, unknown nodes, current tags and hostile counts. The large source-only Targets carrier uses bounded cuts. These carriers are codec tests, not actual original Save observations of missing branches.

Actual original records observe 64 node dispatches. The nine source-only dispatches are Choose Improvement, Has Inquisition, Interval, Is Near Fallout, LUA Script Condition, Launch Inquisition, Operation Military Formation, Run Behavior Tree and Use Scripted Command. Their target reader/writer branches are checked, but no original saved carrier is claimed for them.

An additional external carrier exercises all 73 dispatches with high-bit raw words, extreme shared indices/DataTypes, self/cyclic child and free-list links, aliased entries, duplicate free indices and out-of-range slot metadata. It roundtrips and rejects insufficient shared aggregate item budgets. These values are retained as wire fields; no native graph validity or alias reconstruction is claimed. The grammar has finite statically bounded type nesting and no recursive node field, so hostile child/index values cannot create parser recursion. A separate configurable recursion-depth limit is inapplicable to this codec.

## Remaining native boundary

Targets Read performs player resolution, barbarian-dependent callback selection, signal/archive initialization and live definition-dependent loading. Manager Read resolves the Player and initializes lists/archive; tree Read uninitializes/reinitializes the live object and synchronizes definitions to Blackboard. SharedData pointers, callbacks, subscriptions, scheduler behavior and initialized database/world dependencies are not reconstructed here. Absence of saved pointers proves no semantic irrelevance, planning-only status, or transitive RNG isolation.

All player decisions must still yield to external policies. Neither synthetic cyclic wire references nor saved native behavior-tree state authorize executing native AI policy, fulfilling player actions, ending turns or accepting complete state/RNG parity. No codec-source correction was required by this review.

Reproduce from the repository with the existing private fixture files available:

```sh
python3 evidence/linux/player-ai-targets-behavior-review.py \
  --source-commit eb5bbd03bdb9fbf5394a7a45a29240a3ac8b8eb5 \
  --output target/linux/player-ai-targets-behavior-review.json
```

Discovery names retain the existing [cru121 reference attribution](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; the supplied Linux target bytes, symbols and DWARF remain authoritative. The public report contains identities, schemas and conclusions, not proprietary game binaries or project data.
