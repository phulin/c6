# Persistent ordinary ranged combat

`c6_data::loaded_actions::session::Session::range_attack` executes a qualified
ordinary ranged Unit/Unit operation through CanStart, Test, the operation manager,
combat resolution and the stock `HeadlessMessagesProcessed` aftermath. It borrows
the existing world, visibility runtime and reporting provider. Script state, RNG,
callback registrations and modifier stores persist between requests.

This is a bounded Linux XP2 surface. Complete Civ VI state, arbitrary action
execution and complete turns remain unimplemented. The target and rules identity
must match [the supported Linux target](targets.md).

Initialize the existing `melee_attack::backend::Backend` and combat callback
`Bindings` from typed current inputs. Split that backend once using
`loaded_actions::stock_notifications::combat::State::from_backend`. Install the
returned effect and requirement stores in the stock runtime's **one**
`unit_creation::ModifierOwners`, retaining its collection and factory owners.
Bind the combat state with `Runtime::bind_combat`, alongside the current movement,
placement services, script and reports, before constructing the persistent Session.

For requests, compose `ranged_attack::stock::Ranged` with the existing
`move_to::combat::WithCombat`. It computes target legality using the current
`target_selection::Definitions`, action inputs and `transition::Inputs`; it calls
the current stock runtime for resolution. `ranged_attack::stock::Damage` binds
ordinary damage and death to the existing stock Unit kill lifecycle, including
current influence callbacks. Pass that dispatcher to `Session::range_attack` with
an explicit ordinary target request. The external policy chooses the action.

The implementation loans the canonical effect and requirement stores during
synchronous resolution and restores the mutated stores on success or error. It
retains the backend's original World identity, so an independently loaded copy
cannot replace that world. A reached unsupported callback preserves previous
state changes, reports and RNG draws. The Session records that failed prefix and
rejects later action requests; `into_parts` returns the owners for inspection.

The qualified stock output scope replaces native status text, XP display caches
and platform achievement output with typed records. Additional synchronous
consumers require explicit support. Positive unimplemented branches return errors,
including bombard or Unit/District combat in this adapter, combat capture, joint-war XP consumers,
multiplayer combat notifications, positive rewards/yields/boosts and positive era
score continuations. This adapter does not compose melee combat. Definition,
observer, promotion, notification and callback inputs must describe the actual
initialized world; absent inputs are errors when reached.

The retained native representative cases cover an ordinary Archer attacking a
barbarian Warrior that survives, and the same attack killing a wounded Warrior.
Both pass through the public Session to `HeadlessMessagesProcessed`, matching the
compared gameplay state, visibility, callback effects, history, modifier lifetimes
and ordered GameSync draws. A controlled missing-report case also verifies that
the failed prefix and Session freeze survive extraction and reassembly. These
comparisons do not establish arbitrary ranged configurations or complete turns.

## City ranged strikes

`Session::can_city_range_attack` queries an explicit target without consuming a
shot or RNG. `Session::city_range_attack` executes the corresponding ordinary
City command using the actual District attacker and the same persistent combat,
modifier, script, visibility and notification owners. The external policy
supplies the City, optional District and target coordinates.

Bind the stock runtime's city report scope with `Runtime::bind_city_strike_reports`
and supply current `city_range_attack::Queries`, including actual combined group1
visibility. That authority is revision-bound; saved group0 counts do not replace
current shared visibility. The command includes both native end-turn-blocking
updates and the final processed-message callback.

The retained Linux representative case is a City with Ancient Walls striking a
healthy enemy barbarian Warrior: 11 damage, one shot consumed and unchanged
received XP. The complete public Session command matches the compared affected
state, callbacks, history, modifier lifetimes and ordered gameplay RNG. This does
not establish arbitrary City combat, capture, air combat or complete turns.
Defender death, major-versus-major war weariness and other reached unsupported
continuations return explicit errors and preserve the partial transition.
