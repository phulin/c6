# Linux District creation

`district_initialization` constructs the canonical `player_districts::District`
and inserts it into the current owner's retained recycled collection. It uses
the shared source-derived allocator, then assigns the generated ID, location,
parent City and admitted District type. Constructor values include the native
invalid coordinate `(-9999,-9999)`, last-damaged turn `-100`, definition-sized
zero Yield and GreatPersonClass arrays, and incomplete/unpillaged flags.

The Data adapter runs computed `Combat.Restore` before appending the District ID
to the City's ordered membership and setting the actual plot's District ID.
Restore uses the existing current wall/attack-strength query and only assigns
City attacks-per-turn when that computed strength is positive. The enclosing
Player/table owner remains distinct from `District.GetOwner`, which reads the
current plot's CityAttributes player.

`set_complete` writes the canonical flag first. A true assignment checks the
current Game and City, resolves the current owner and City again, dispatches the
Player completion signal, then the Game completion signal. Repeated true
assignments repeat those effects. Definition/reward lookup occurs after both
signals so their synchronous changes are visible. Effects implement these two
specific signal deliveries against the same mutable state; no default success
callback is provided.

The current Player.Units reward lookup includes the DistrictReplace fallback.
Absent maps and zero-count entries are computed from canonical state. Positive
unit awards currently return a typed `FoundingUnitAward` error after preserving
the flag and preceding signals; unit/GreatPerson creation and decrement effects
remain unsupported at that leaf. Missing inputs and subscriber errors likewise
retain the actual reached prefix.

The implementation follows Linux XP2 constructor `291d4b0`, Initialize
`291e020`, Player.Create `2ae11c0`, City.Create `2884e10` and SetComplete
`291e780` in the pinned target. It does not claim complete District placement,
completion subscribers, FoundCity execution or action/RNG parity.
