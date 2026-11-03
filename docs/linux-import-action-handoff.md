# Loaded state to SkipTurn and Manager

The immediate goal is a mutable gameplay state initialized faithfully from the
decoded records. Civ VI save production is not required. Byte roundtrips are
cheap schema diagnostics; byte-identical rewriting is not an action-acceptance
gate. The contextual current-wire result in
[the import profile](linux-core-contextual-import.md) supplies all 43 saved
Player records, including their complete ordered Unit/Formation collections.
It does not yet supply native Read initialization or all unsaved controls.

| Decoded input | Existing consumer | Required application |
| --- | --- | --- |
| `CorePrefix.map` and its layered Unit references | `world::World::new`, `plot_units`, placement/movement queries | Move the Map into owned state; retain native layer/cell/reference order. Rebuild only source-proven derived indices. Unsaved City domain masks still require the verified registry/definition rebuild inputs. |
| Each `FirstPlayer.header.owner`, Units table and Formations table | `world::Player::new`, `unit`, `unit_formation`, `members` | Move records and allocator metadata, converting the two slot wrapper types without changing IDs, owners, slot order or free links. Existing World rejects owner/slot inconsistencies and keeps requested versus resolved IDs. Do not manufacture the Manager's 65-slot validity mapping from these records. |
| `Unit.i_operation_timer`, `e_owner`, `id`, ordered `k_operations` | `ManagerUnitState`, `is_busy` | These pure reads already use the actual typed Unit. Preserve queue order, operation hashes and parameter values. Runtime handler IDs still require the genuine operation registry. |
| `Unit.x_moves_remaining`, activity, fortification, automation, movement-completion flags | `can_move`, `skip_turn_can_start`, `skip_turn::replace_operation` | Read the actual fields; do not supply precomputed busy/movement answers. The replacement kernel has a bounded MoveTo/single-member phase and explicitly rejects other queues, automation and pending movement completion. |
| `Unit.formation_id`; Formation IDs, owners, ordered members, path and cache fields | `Player::unit_formation`, `FormationEffects`, path/cache clearing | Use original low16 lookup and actual resolved identity; do not require generation equality where native Find does not. Formation's unsaved evaluating-operation state remains a separate initialization requirement. |
| `PlayerHeader.ordered_flags[0]` and `[2]` | `Player.is_alive`, `c6_data::unit_operation_manager::WorldPlayer::is_turn_active` | These are saved IsAlive and TurnActive, respectively. Keep their saved provenance until the relevant Read/post-load effects are applied or checked. SlotStatus/human status and team come from Configuration, not these flags. |
| `CorePrefix.current_turn`, `local_player`, `phase_processor` | request inputs, notifications, `PhaseDefinitions::from_processor` | Pass the actual saved values with their source identity. Select the phase definitions from actual configured TurnPhaseType and native definition order; no guessed single-player mode or permissive tactical flag. |
| `Unit.k_operations_disabled` | `OperationDefinitions::restore_unit_disabled` | Existing code implements fresh Reset(false), actual-count sizing and ordered hash remapping. Bind it to the genuine native-order definitions, not SQL ordinals; later gameplay changes must update live storage. |
| Player Techs/Culture acquired arrays | `player_prerequisites::IndexedBools`, owned Player technology/civic state | Use the existing source-proven reset/remap application with actual definitions. Saved global node data is not automatically post-Validate availability. |

This can be assembled without cloning the whole imported world. The import
result currently owns the Player records; a consuming assembly adapter should
move Map, Unit/Formation vectors and retained owner components once, build the
existing dense indices, and preserve any still-required saved components in an
owned imported-state aggregate. It must not discard City/diplomacy/upkeep data
because the first action only accesses Units. Stable typed handles replace
native pointers; allocator addresses/capacity are not an independent gameplay
goal, while logical allocation/identity and query ordering remain exact.

The next concrete loading implementation dependency is the current Unit28 and
Formation1 Read/application postlude, together with owned Manager membership:

1. Preserve the decoded Unit definition **hash** (`e_type_index` is its wire
   name) and resolve the actual initialized Unit definition in the admitted
   rules collection. It is not already a collection index. Implement the
   source-verified domain/definition/reference initialization and array-overlay
   defaults; unknown hashes or absent initialization inputs remain errors.
   The generic definition-array reader consumes unknown keys and overwrites
   duplicate known keys in order; it does not universally clear destinations.
2. Initialize the logical archive identities and registration/dirty behavior
   needed by the actual Unit/Formation setters, rather than setting the
   replacement kernel's five `*_archive_bound` inputs to true. Native pointers
   and facade locks are not Rust state, but their gameplay-affecting ordering
   and report effects must be preserved.
3. Admit or reconstruct actual Manager player mapping and nonnull-slot facts,
   Configuration slot status/team, operation/phase definition bindings and
   omitted live pause/automation controls. A complete saved roster alone
   supplies none of those missing authorities.

[World ownership](linux-world-state.md),
[Unit definition identities](linux-unit-definition-ids.md),
[Unit/Formation loading limits](linux-unit-codec.md),
[definition-array application](linux-unit-wire-contracts.md) and
[concrete Manager providers](linux-unit-operation-manager-world.md) pin these
distinctions to existing target evidence. A canonical Save with matching Unit
bytes does not by itself attribute an unrelated live registry/configuration
capture to this load; adapters must check their actual profile identities.

Complete SkipTurn also needs the existing state-backed friendly-count and
plot-limit inputs, actual `Game.DiplomaticRelations` war storage (not
Player.Diplomacy), handler interruption/dispatch bindings, real scripting
callback effects and lock semantics, and ordered reporting/diplomacy/
notification continuation. Notification expiry can use the decoded current
turn; omitted live notification blocking state and actual subscribers cannot
be replaced by defaults. The current source providers identify these inputs
explicitly; the smaller mutation kernel's `activation_can_start` boolean is
not a substitute for executing the full Manager predicate.

The older OperationDef5 input gap is independent of current-format action
work. It remains unsupported, without coercion or blocking state-backed action
integration. External policies still supply every genuine player choice;
native AI planning and desktop/UI admission are not prerequisites for the
gameplay boundary. Gameplay upkeep, NPC effects, complete RNG and all
gameplay-affecting callbacks remain required.
