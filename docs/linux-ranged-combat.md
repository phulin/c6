# Persistent ordinary ranged combat

`c6_data::loaded_actions::session::Session::range_attack` executes a qualified
ordinary ranged Unit/Unit or supported city-center operation through CanStart, Test, the operation manager,
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
including bombard, unsupported District configurations, combat capture, joint-war XP consumers,
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

## Unit ranged attacks against Ancient Walls

The same `Session::range_attack` and `ranged_attack::stock::Ranged` dispatcher
support an ordinary land ranged attacker against a major-owned city-center
District. Supply `melee_attack::city::Definitions` in
`execution::GenerateRules::city`, bind the current combat placement queries and
same-origin District damage report scope, and retain the current technology and
civic graphs for notification aftermath. The city remains a real District
participant; it is never represented by a synthetic defending Unit.

Generation registers a RANGED proposal and consumes the original inner then
outer damage draws. Resolution applies outer damage before garrison damage,
awards experience, clears operations, updates healing eligibility and attack
budget, and executes war weariness, combat callbacks and
`HeadlessMessagesProcessed` on the existing owners. It preserves both defense
health pools and the generated pre-damage defense type. Exhausting the outer
pool removes the wall strength contribution from subsequent current queries.
The ranged attacker takes no retaliation damage and never advances or captures.

Two retained original Linux fixtures use an Archer and a noncapital city with
Ancient Walls. With undamaged walls, the city receives 1 garrison damage and
14 outer damage. With 99 of 100 outer damage before the action, it receives
25 garrison damage and the outer pool clamps to 100. Both complete through the
persistent Session with the compared gameplay state, callbacks, history,
visibility and ordered GameSync draws equal. The source-qualified damage report
order is outer then garrison. Read-only post-action tests reject another ranged
attack after its attack budget is exhausted; current wall strength loses its
3-point contribution when the outer pool reaches 100.

The existing city-melee planning boundary also applies: the library emits
`CityAttackPlanningObservation` instead of updating the native targeting tally.
The comparison retains the raw 0-versus-2250 tally difference and checks all
other Targets fields. This source-qualified policy exclusion permits no
omission of gameplay or RNG from other AI-named code.

The final comparison uses public gameplay commit
`e6a8b10bdda682c8965947bad58c0eae17ac1b87` and private adapter commit
`6dfcab2802344d41b30cacc322c8aa0b4b9dcdb1`. Private receipts
`evidence/linux/city-ranged-walls-session-4.json` and
`evidence/linux/city-ranged-walls-exhaust-session-3.json` bind the canonical
inputs, target/rules/configuration identities, executable hashes and commands.

Urban Defenses, bombard/equal-type attacks, positive bypass-wall support,
special wall-prevention modifiers and lethal garrison continuations are explicit
unsupported branches. Other unqualified callback, promotion and joint-war
branches retain the existing errors. These fixtures do not establish arbitrary
city combat, all RNG sources, complete gameplay state or full turns.

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
