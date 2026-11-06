# Ordinary Settler capture

The persistent Rust `Session` supports a bounded ordinary Warrior MoveTo into an
unescorted hostile Settler. The caller supplies the order and current capture
services through
[`civilian_capture::Movement`](../crates/c6-data/src/civilian_capture.rs).
The action uses the existing legal request, target selection, combat generation,
delayed death, unit factory, placement, sight and operation owners. This is a
representative Linux action, not complete civilian capture or full-game support.

The compared case has a human, cityless captor at war with another major player.
Warrior `(0, 131073)` moves from `[17, 11]` to `[18, 12]`, capturing an undamaged,
unembarked Settler with zero build charges. The conversion comes from the active
`UnitCaptures` rules: Settler becomes Settler. The old `(1, 196610)` remains in its
owner's roster with delayed death and an invalid location. A fresh `(0, 196610)`
is created at the destination, retains original owner 1, has zero movement and
is marked processed for the turn. Both identities remain distinct despite their
equal numeric IDs. The Warrior ends with zero movement and no queued operation.
The victim retains another Settler and Warrior; player elimination is outside
this case.

Capture runs before the Warrior's placement. The native civilian combat-generation
branch performs its strength and member finalization queries without ordinary
combat proposals or damage draws. The action preserves capture mayhem, removal
and creation effects, current DefaultDefeat requirements, modifier and collection
memberships, placement callbacks and operation cleanup. The reached AI capture
listener checks the human flag before creating an operation or drawing RNG; no
player policy is selected. The current Session gossip history is empty in this
case, so the reached Settler-created gossip callback takes its empty-history
return. Other reached branches require their own implementation.

The single MoveTo completes through `HeadlessMessagesProcessed` without advancing
a turn or creating a pending decision. The comparison matches every typed Map,
Unit and Formation component and roster, represented Player gameplay fields,
core processors, global religion, GameSync and tracked libc state on the
[pinned Linux target](targets.md). It also matches the affected unit objects,
seven modifiers with their effect parameters and tracked/applied memberships,
ordered callback rings, collection reference counts, logical allocation counters,
and the affected DefaultDefeat requirement states, IDs and references.

Raw Player comparisons retain differences between imported AI runtime validity
flags and fields absent from native Save output. Unknown bytes remain retained.
The reports do not claim complete runtime graph coverage, a complete ordered RNG
inventory or full action equivalence. The native fixture and exact public/private
pins, configuration, commands, inputs and actual results are retained in the
private Linux civilian-capture comparison records.

Retreating civilians, other unit types, damaged or embarked Settlers, nonzero
charges, barbarian captors, captors with cities, nonhuman capture policy and
positive Settler-created gossip history remain outside this profile. Unsupported
continuations return explicit errors with completed mutations still visible.
