# Ordinary Settler production

The persistent Rust `Session` supports ordinary Settler production completion
through bounded Linux turn progression. This is representative early-game
coverage, not complete Civ VI production or full-game turn support.

An external policy selects production with `Session::choose_unit`. The stock turn
services borrow the ongoing unit factory, city workers, rules, requirements,
Script and callback owners. Completion creates and places the unit, executes
creation subscribers, consumes the city population, updates worker assignments
and religion, and finishes the queue and overflow. The incoming player's real
research, production and unit decisions remain external.

The checked original fixture starts with a population-two city and an originally
legal Settler queue, one Q8 production unit below its actual cost of 80. Original
population and production APIs prepare that initial state; the captured action is
ordinary turn completion. The same Session produces Settler 196608 at its city's
location, reduces population to one, empties the queue, clears unit progress and
retains overflow 1279 Q8. The new unit receives the same turn's normal movement and
healing-eligibility restoration. No completed unit is supplied as simulator input.

Compared typed gameplay components, unit/formation rosters, player gameplay
fields, city population/yields/workers/queue, represented requirement callbacks,
core processors and tracked global RNG/libc state match the pinned
[Linux target](targets.md). The run ends at owner 0's next genuine decision
boundary. Complete RNG/runtime coverage remains unestablished; raw policy/control
and wire-annotation differences remain recorded.

The supported case has ordinary formation, the current stock subscribers and
empty current gossip-visibility history. Unknown or positive unsupported
creation, gossip-delivery, modifier or yield-grant branches return explicit
partial-transition errors. Reached mutations remain visible after such an error.
