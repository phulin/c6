# Persistent ordinary melee combat

`c6_data::loaded_actions::session::Session::melee_attack` executes an ordinary
queued MoveTo attack through current target legality, combat resolution and stock
`HeadlessMessagesProcessed` aftermath. The external policy chooses the explicit
request. The action borrows the persistent world, visibility, script, RNG and
modifier owners; it never delegates policy selection to native AI.

This is a bounded Linux XP2 surface. Complete Civ VI state, arbitrary actions and
complete turns remain unimplemented. Inputs must match the
[supported Linux target](targets.md).

Initialize combat using the shared backend and callback bindings described in
[the ranged combat setup](linux-ranged-combat.md). Retain a
`melee_attack::stock::Movement` referencing the current path definitions and
placement queries. Assign it to the combat state's `melee_queries` before binding
the state on the stock runtime. These queries compute combat movement cost,
visible enemy presence and defensible districts from the current world.

Compose `melee_attack::stock::Melee` with `move_to::combat::WithCombat`, supplying
current target definitions, transition inputs, the same movement query view and
`melee_attack::stock::Damage`. The damage adapter runs the existing stock Unit
kill lifecycle and influence callbacks using the canonical requirement and effect
stores. Pass this dispatcher to `Session::melee_attack` with an explicit target
request. The containing MoveTo operation can continue after combat.

For a [major-player defender killed through a retained ordinary round](linux-wartime-lethal-melee.md),
compose `melee_attack::stock::RetainedDamage` around `Damage`, with the live
default-defeat bindings and current movement definitions. This composition also
loans the same creation collections during death callbacks. Keep the canonical
requirements around the action with `Session::with_modifier_requirement_sets`.

Resolution restores its borrowed owners on success or error. Reached unsupported
branches preserve earlier gameplay updates, callbacks and RNG draws; the Session
then freezes and rejects later requests. Callers can extract its owners for
inspection. Native status text and XP display output become typed records in the
qualified stock scope.

[Ordinary land melee against a surviving city](linux-city-melee.md) is also
supported in its no-wall profile. Other District/religious combat, combat-unit
capture, multiplayer combat notification
creation and positive unimplemented rewards, yields, boosts or era-score
continuations return explicit errors. Missing reached inputs also return errors.
These limitations do not silently complete the action.

Retained Linux comparisons cover Warrior/Warrior combat with a living defender
and a wounded defender killed by the attack. Both finish through the public
Session and match the compared current gameplay state, visibility, callback
effects, history, modifier lifetimes and ordered GameSync draws. [Clearing a stock barbarian camp](linux-camp-clear.md) through a wounded Scout
kill is also compared, including its positive civic boost and tribe updates.
These are representative profile comparisons, not arbitrary melee or complete-turn parity.
