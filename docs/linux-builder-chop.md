# Ordinary forest Chop

[`Session::chop`](../crates/c6-data/src/loaded_actions/session_chop.rs) accepts an
externally selected Builder action through the persistent Session's operation
queue, manager and stock aftermath. Supported native cases use an ordinary,
owned, resource-free, unimproved forest on passable terrain.

The first native case grants 21 production and spends charge 3→2. The compound
case starts with one Builder charge and an ordinary Monument queue one Q8 unit
below its cost. Chop grants 21 production and completes the Monument inside the
action's production callback. Its building, yield, loyalty, statistics, modifier
and requirement effects execute on the same canonical state and owners. An empty
queue publishes a production decision for the external policy.

The final charge invokes delayed destruction. Placement, sight and reached Unit
modifier callbacks are removed immediately. At the native message boundary the
Builder still occupies its roster entry, has delayed death and retains its queued
operation. Physical roster retirement happens later; it was not exercised by this
compound case. The surviving Builder's FinishMoves path is not substituted here.

Callers provide current completion and death services through the Chop dispatcher,
with the Session's existing city-query, visibility, movement, modifier and
requirement owners. Missing or unsupported services return an incomplete action
and preserve the reached mutation prefix. Building attachment errors retain their
nested cause.

The compound Linux comparison matches the complete compared City and queue,
player fields, Unit/Formation/map state, processors, territory/yield owners,
building counts, two reached Monument modifier states and requirement identities,
Builder cleanup, all four visibility owners and the stated RNG owners. A single
optimized action sample took 1.636 ms, excluding import, initialization and
comparison. This is a bounded action measurement.

Other feature/resource combinations, positive faith or gold harvest continuations,
additional callback consumers, arbitrary production and complete RNG inventory
remain unsupported or unqualified. Complete Civ VI state, arbitrary actions and
full turns remain unimplemented.
