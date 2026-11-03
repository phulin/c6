# Linux Player AI suffix: state and policy triage

The fifteen AI-owned records in Player v47 remain required, unclassified state until their fields and consumers establish a narrower role. **No whole-block exclusion is proven by this audit.** Their namespace, analysis-like field types and omission of some live fields from Save are insufficient grounds for exclusion. Complete gameplay state/RNG, externally supplied player decisions, action execution and turn equivalence remain unimplemented.

The [audit](../tools/linux/player_ai_state_roles_audit.py) and [report](../evidence/linux/player-ai-state-roles.json) gate complete method extents against Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. The report records resolved direct call edges, indirect sites, reader initialization dependencies and selected update consumers. It executes no original game code. Pinned [cru121 reference attribution](reference.md), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, applies to discovery names; supplied target symbols, DWARF and bytes are authoritative.

Player writer `0x2b4ee00` places these records after Governors, CulturalIdentity, Heroes and the inline Unit/Feature/Continent bool hash arrays. They are **unframed**: the next record starts only after decoding the preceding record's actual schema, including nested containers. Existing owner cuts are validation evidence, not a production parser strategy. The inline Player agenda/tail follows BehaviorTree.Manager.

| Order and component | Version | Concrete writer / reader | Structural dependencies and disposition |
|---|---:|---|---|
| 1 Control.Manager | 2 | `0x201cf80` / `0x201d0c0` | Player identity, priority, recycled Control.Instance container, bools, counted polymorphic contracts and null sentinels; contract dispatch schema still required. |
| 2 InfluenceMap | 2 | `0x2140e00` / `0x2140e50` | Two serialized short vectors, `m_influence` and `m_religiousInfluence`; historical reader gates the latter at v2. Per-cell values and their consumers remain required/unclassified. |
| 3 Planning.Manager | 1 | `0x21b7560` / `0x21b75b0` | Exactly eight bytes: version and Player identity. Closed below. |
| 4 CityManager | 2 | `0x1fcf530` / `0x1fcf5a0` | Version constant at `0x2dfbc20`, Player identity, recycled CityBuild container. Nested CityBuild schema remains required. |
| 5 CultureAi | 15 | `0x2028c60` / `0x2028e20` | Control handles, BoostManager, flags, indexed PseudoYield/GovernmentSlot/Civic/Policy/Resolution/Government values, RockBandAnalysis, GreatWorks and trailing values. |
| 6 Diplomatic | 18 | `0x205b820` / `0x205ba90` | Player identity, fixed-point relationship vectors/matrices, StateVector, ActionEvalVector, control handles, definition-indexed DiplomaticAction lists and other state; diplomacy cannot be blanket-excluded. |
| 7 Economic | 4 | `0x20a3110` / `0x20a3260` | Identity and heterogeneous wrapped/container fields; nested contracts and consumers remain required. |
| 8 Governors | 3 | `0x2115ba0` / `0x2115c40` | Player identity, Governor/GovernorPromotion analyses and wrapped state; distinguish analysis from actual Player.Governors. |
| 9 GrandStrategic | 12 | `0x211e180` / `0x211e2b0` | CityPlanner, flags, VictoryStrategy, indexed PseudoYield/Yield values, UnitProduction; Read initializes site evaluation and signals. |
| 10 Homeland | 1 | `0x2134880` / `0x2134910` | Identity, control-instance ID, priority and MoveList; Read reconstructs Control callbacks and subscribes Player signals. |
| 11 Military | 14 | `0x21453f0` / `0x2145620` | Scalars, bool vectors, technology hash, indexed AiOperationType values, recycled Operation container, fixed array, MilitaryDefense and other indexed values. Operation state/consumers remain required. |
| 12 Research | 4 | `0x2183d20` / `0x2183e40` | Player identity, technology hash, indexed Unit/Technology values, TechTreeAnalysis, bool, control handles, BoostManager and int; chosen research crosses into Player.Techs. |
| 13 Tactical | 2 | `0x2195480` / `0x2195510` | Identity, control-instance ID, MoveList and UnitAI type; Read reconstructs callbacks and subscribes Player signals. |
| 14 Targets | 4 | `0x21a79c0` / `0x21a7a80` | Identity, indexed TargetType bools, CivKnowledge, PlotCoords, exploration maps and TargetList. Knowledge is not automatically planning-only. |
| 15 BehaviorTree.Manager | 1 | `0x26e4060` / `0x26e40e0` | Identity, Blackboard and recycled BehaviorTree container. Blackboard entries and callback consumers require field-level review. |

This is a dependency inventory, not a complete grammar for the fourteen remaining blocks. Their reported versions are emitted by the current writers; legacy compatibility branches are not a supported codec. The report also gates operator wrappers where a writer delegates to a member method, preserving the distinction between wrapper RVA and concrete implementation.

## Closed bounded record: Planning.Manager identity

The current writer emits little-endian `u32(1)`, then `i32(player_id)` obtained from `m_player->GetID()`. No count, padding, unit planner record, priority, RNG object or pointer bytes are emitted. Thus its exact extent is eight bytes regardless of the live `m_unitPlanners` contents. Matching DWARF identifies `m_Initialized` at offset `0x98`, `m_player` at `0xa0`, wrapped `m_priority` at `0xa8` and wrapped `m_unitPlanners` at `0xe0`.

The reader consumes a version word without branching on it, reads the signed ID, calls `Globals::GetPlayer(id)`, stores the returned pointer at `+0xa0`, calls `InitializeSignals()` and `InitArchive()`, then sets `m_Initialized` to true. The proposed current-version codec should still reject unsupported versions explicitly. It must preserve identity and must not claim to reproduce the native reader merely by retaining the eight bytes.

`InitializeSignals` (`0x21b7600`, 275 bytes) subscribes to Player turn activation/deactivation through the corresponding Manager callback slots. `InitArchive` (`0x21b7720`, 175 bytes) uses participation/identity and component identity `0x3d` to establish archive/logging integration. It contains indirect logging calls. Consequently absence of a direct RNG call does not establish transitive purity or identical constructor/load lifecycle. Lookup requires the genuine global player mapping; a signed ID alone is not a reconstructed Player object.

The audit independently checks the existing two canonical Core streams and their SHA-gated owner traces. All **86 Planning.Manager records** are eight bytes, have version 1, exactly reconstruct through typed scalar packing and match the enclosing Player v47 identity. This is structural/identity validation of existing original Save outputs, not a new original Read oracle, a production Rust codec or gameplay acceptance. The unsaved priority/planner vectors remain omitted live-state dependencies; this finding does not classify them as semantically unnecessary.

## Policy, gameplay consumers and RNG

Policy execution must yield to the external decision supplier. For example, gated `Research::DoTurnActivate` (`0x2185a70`, 1693 bytes) reads Player.Techs, scores TechTreeAnalysis values and calls `QueueNextTech` at call site `0x2185eb0`. The latter (`0x2186110`, 764 bytes) is separately gated: it checks IsHuman, edits Player.Techs and calls `SetResearchingTech` at `0x2186224` and `0x218638d` (body `0x2b8ba90`). Its fallback invokes `WeightedVector<TechTypes>::ChooseAtRandom` at `0x2186360`, using a delegate constructed from Globals.GetGame and `Game::Instance::GetRandNum`. The audit verifies the GOT relocation `0x3b1c648` to callback body `0x1fe5240`; that 39-byte wrapper edits the Game.Random archive variable at `+0xe8` and tail-calls `Random::Get`. Thus this is a concrete original-game RNG consumer in policy selection, followed by gameplay mutation. Original human/control flags must remain unchanged: marking every player human would alter gameplay gates and potentially handicaps. Which draws must occur when replacing policy with an external choice still requires a defined and validated stream/caller contract; this audit neither preserves nor drops those draws. This is a useful policy-to-gameplay boundary to investigate; it does not authorize automatically choosing the next technology. Player research progress, legal choices and all exact upkeep remain in scope.

Planning.Instance `DoCombatSimulations` (`0x21b7d20`, 1237 bytes), also gated, calls `Combat::Manager::GenerateCombatResults` at `0x21b7ef9`, along with district/unit/combat accessors. This is a specific warning against inferring RNG isolation from the planning namespace: whether those simulation arguments isolate gameplay state and RNG must be proven through that callee and its transitive consumers before excluding the work. No native simulation is run here.

The report screens **direct** edge names, resolving PLT bodies where available for Random/RNG calls and records unresolved indirect sites. It does not compute a complete transitive call graph, prove no RNG consumption during Read/update, classify streams, or reproduce RNG state. Nested serializers, Initialize/Reset/InitializeLists helpers, message callbacks, polymorphic contracts, operation execution and logging/service callbacks remain dependencies. Empty direct-name results are not a passing RNG comparison.

Reader side effects vary: Control and CityManager call Initialize; CultureAi resets analyzers; Diplomatic initializes relationship lists, DiplomaticState signals and archive; Research initializes lists and marks its analyzer updated; Homeland/Tactical reconstruct Control callbacks. Treating all these readers as plain byte copies would omit real effects. All unknown bytes stay unclassified and unmasked, including the existing **89,198-byte raw Save→Load failure**.

Reproduce without game execution:

```sh
python3 tools/linux/player_ai_state_roles_audit.py --output target/linux/player-ai-state-roles.json
```

Validation here is the SHA-gated static audit plus existing original fixture identity/extent comparisons. No native runner, execution configuration, gameplay codec or Rust source changed; full world import, complete Player state and state/action/turn parity remain open.
