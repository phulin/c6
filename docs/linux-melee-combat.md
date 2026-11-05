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

Resolution restores its borrowed owners on success or error. Reached unsupported
branches preserve earlier gameplay updates, callbacks and RNG draws; the Session
then freezes and rejects later requests. Callers can extract its owners for
inspection. Native status text and XP display output become typed records in the
qualified stock scope.

District/religious combat, combat-unit capture, multiplayer combat notification
creation and positive unimplemented rewards, yields, boosts or era-score
continuations return explicit errors. Missing reached inputs also return errors.
These limitations do not silently complete the action.
