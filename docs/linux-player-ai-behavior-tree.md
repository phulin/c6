# Linux Player AI BehaviorTree saved state

The bounded [codec](../crates/c6-core/src/player_ai_behavior_tree.rs) covers the current Linux XP2 `AI::BehaviorTree::Manager` saved grammar, its distinct Blackboard, recycled BehaviorTree entries and all 73 built-in NodeData dispatch identities. It does not execute policy or reconstruct live game objects. Full Player/world import, gameplay state/RNG, actions and turns remain unsupported. No AI-owned block is excluded as planning-only, and the recorded **89,198-byte raw Save→Load difference remains unmasked**.

The read-only [audit](../tools/linux/player_ai_behavior_tree_audit.py) gates target SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, derives node identities and writer branches from that ELF, checks target DWARF and compares existing original Save outputs. It executes no original code and writes no private project artifacts. The [report](../evidence/linux/player-ai-behavior-tree.json) records method identities, complete schema bindings and fixture/candidate identities. [cru121 reference attribution](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, applies to discovery names; actual target symbols, DWARF and bytes establish these contracts.

## Current grammar

All integers use little endian. Counts are unsigned 32-bit words. Sequence ordering, duplicate values/labels, signed raw words, enum/hash bits, string bytes, raw bool bytes and float bits are preserved. No normalization, sorting or opaque skip occurs. Unsupported versions, the current fixed DataTypes count other than 12, recycled block size other than 16, and unknown unframed node hashes fail explicitly.

| Record | Current writer / reader | Wire order |
|---|---|---|
| Manager v1 | `0x26e4060` / `0x26e40e0` | Version, signed Player ID, Blackboard, recycled BehaviorTree container. |
| Blackboard v3 | `0x26c38f0` / `0x26c3a00` | StringTable; Units, Techs, Buildings, Ints, Bools, Plots, Strings, Improvements, IDs, FixedPoint Q8, Districts, Civics BlackboardVectors; signed job index. |
| StringTable v1 | `0x26c3840` / `0x26c38a0` | Version, counted ordered DataEntries. |
| DataEntry v1 | `0x26c3700` / `0x26c3790` | Version, label hash, current DataTypes count 12, twelve signed indices. |
| BlackboardVector v1 | Instantiated serializers in report | Version, counted BlackboardData entries, signed first-unused index. |
| BlackboardData v1 | Instantiated serializers in report | Version, typed value, signed next-unused index. These are free-list links, not reference counts. |
| BehaviorTreePriority | Seven concrete bindings in report | Definition hash (or raw ID for unsigned specialization), signed weight, signed priority enum. |
| SharedData | 24 concrete Write/Read bindings in report | Signed DataTypes word, unsigned index. No pointed-to bytes or pointer address. Actual saved values belong to Blackboard. |
| BehaviorTree v5 | `0x26b38d0` / `0x26b3a60` | Node count, signed Player ID, control ID; five bool bytes; name and owner-name narrow strings; request/recruit priorities, tree ID, operation ID, present-value Q8 word; GameModifierAnalysis; counted ActionStates, DecisionStates, NodeDefinitions, ControlHandles. |
| DecisionState v1 | `0x26e9eb0` / `0x26e9f20` | Version; FirstChild, Jump, LastChild, Parent, StartAt unsigned words. |
| NodeDefinition v3 | `0x26e9f80` / `0x26eaf70` | Version, shape enum, NodeDefinition hash, NodeData v33 and its type-selected fields, unsigned NumSharedData. |
| NodeData v33 | `0x26e9ff0` / `0x26eb070` | Version, fields dispatched by the actual initialized node type. Complete per-node C++ bindings are retained in `NODE_SCHEMAS` and independently derived by the audit. |
| Recycled container v`0x10001` | `0x26e6370`; reader binding in report | Version, block size 16, capacity, live-entry count; each signed slot index and BehaviorTree; current-ID bits, free count, first/last-free words, then free-index sequence. |

Blackboard priority values are counted vectors of the corresponding priority type; ints and Q8 values are raw signed words; bools are bytes; PlotCoord is two signed words; narrow strings have a byte count followed by exact bytes. StringTable's twelve fixed indices are charged to the aggregate item budget because they have an explicit saved count. Fixed schema members and NodeData fields have no sequence count and do not add dynamic items. Every counted container, free-list element, priority value and narrow-string byte does.

`InitializeBehaviorTreeNodeIndices` (`0x26c2a60`, `0x7ff` bytes) binds 73 exact string literals through `Utilities::MakeHash` and `Types::GetIndex`. The audit follows each matching NodeData writer branch, retaining its ordered serializer calls. It independently walks all 73 current-v33 reader branches and verifies their typed field sequences match the writer, including historical reader branches skipped at v33. The hashes use the verified MakeHash CRC convention and original literal casing. It does not assume rules indices match hashes. Nine built-in dispatches are absent from the supplied corpus; all 73 have small synthetic codec carriers. Native fallback for an unrecognized type is not used to guess an unframed extent.

GameModifierAnalysis v17 is fully typed here; its independent [contract](linux-game-modifier-analysis-contract.md) records 22 definition hashes, scalar and raw float storage, UnitList, collection metadata and five raw-presence conditional object identifiers. The live references require real Player/City/District/Map/Unit lookup. Preserving their identifiers does not perform those lookups.

## Actual records and validation

The two SHA-gated canonical Core streams each contain 43 actual `GameCore::AI::BehaviorTree::Manager` owner hooks nested directly under `GameCore::Player::Instance`. The independent grammar consumes all **86 records, 3,491,036 bytes and 274,462 aggregate items**, with extents from 1,339 to 177,283 bytes. It observes 64 distinct built-in node types. Every typed field, nested value, free-list link and raw storage value is compared against Rust's complete 17,989,602-byte typed Debug projection; re-encoding must reproduce every original byte. Owner offsets are comparison evidence only; production parsing consumes actual schemas and caller input.

The following inline PlayerTail begins at each actual BehaviorTree.Manager end and ends at its enclosing actual Player end. The audit independently projects all 86 tails through the established PlayerTail grammar, covering 84,896 bytes. The established source/evidence checkpoints `4470af9` / `63c16f1` use this same actual boundary. All supplied Players have WasEverAlive=true; no false-branch fixture is claimed.

Candidate `e95b2b3` passes validation using an unchanged Git archive that already exports this committed module. Missing exports fail; source is never injected. All archived Rust source hashes are recorded before/after. Focused actual Cargo tests and strict Clippy run in that archive. Actual records receive exact byte/item-cap checks, one-short aggregate caps, strict trailing rejection, prefix accounting, 14,198 bounded structural/interior truncations and 2,752 hostile version/fixed-header plus 2,752 hostile count mutations. Small carriers cover every dispatch, every raw bool byte, non-UTF-8/nul string bytes, NaN float payload bits, all 32 conditional-reference combinations, unknown dispatch refusal and exhaustive carrier truncations.

Reproduce after selecting the committed exported source checkpoint recorded in the report:

```sh
python3 tools/linux/player_ai_behavior_tree_audit.py --source-commit e95b2b3 --output target/linux/player-ai-behavior-tree.json
```

## Native Read remains separate

Manager Read resolves `Globals::GetPlayer`, reads Blackboard and recycled trees, then calls `InitializeLists` and `InitializeArchive`. InitializeLists uses AI personality/leader/civilization/handicap and gameplay database queries, rebuilds triggered-tree lists, and subscribes Player signals. InitializeArchive establishes the component archive identity and logging integration. These unsaved dependencies are required for native reconstruction; no transitive RNG-purity proof is claimed.

BehaviorTree Read first uninitializes the live object, resolves its Player and Manager, initializes the object and tree identity, reads saved fields, then calls `SyncDefsToBlackboard`. Native callbacks, subscriptions, archive state, gameplay database bindings, pointer resolution and scheduler/execution behavior are not produced by this transport. Blackboard's live jobs/indexer and SharedData storage pointers are not serialized. Their absence from Save establishes neither semantic irrelevance nor a planning-only exclusion. All external player decisions must continue to yield to the external policy supplier; this codec never chooses actions or advances turns.
