# Linux FoundCity entry

`c6_core::found_city` implements the pinned Linux handler's ordered `CanStart`
and `Start` logic. `found_city::placement` computes the stock legal-location
checks and current-map minimum-distance scans. `c6_data::found_city` supplies
admitted `Units.FoundCity`, `Features.Removable` and `Features.Settlement` values
and reads the existing canonical `Player.Units.no_settling_cities` flag.

These pieces do not yet compose a complete founding action. Manager/script
legality, a real City constructor and allocator, placement effects, naming,
capital/palace changes, CityCreated subscribers, religion and delayed settler
removal still require their canonical implementations. Mandatory mutation hooks
have no default no-op implementations. A caller cannot create an exact City by
copying or defaulting a wire record. No native FoundCity differential is claimed.

The source operation hash is `5733ac6c`; its numeric operation index belongs to
the actual initialized registry. FoundCity uses the actor's current location.
The visible-only legality mode skips the placement gate but retains the settling,
unit-definition, movement and plot checks. An unremovable feature does not itself
reject handler legality; placement separately checks `Features.Settlement`.

`Start` runs feature reporting before feature removal and reads the actor's
location again after subscribers. It reads the player's majority religion before
creating the city. A null City creation result skips only the conditional pressure
addition: the original still reports founding and consumption, destroys the unit
later and sets `StartData.action`. It leaves `StartData.delete` unchanged. Callback
errors preserve their already-applied prefix and do not report successful action
completion.

Placement checks current map/plot state and definition values, never captured
legal answers. Cities within the inclusive minimum range reject on the same
landmass; on different landmasses only distance strictly below the range rejects.
The stock Rules.Cities result is checked for valid identity, followed by the
original second scan. A finite probe limit bounds pathological input work;
exhaustion is an explicit error rather than a legal or illegal answer.

Source anchors are `FoundCity.CanStart` at `2d9e1a0`, `FoundCity.Start` at
`2d9e2b0`, placement at `28bda20`, and the stock minimum-distance query at
`2bd57d0`, all in the pinned Linux XP2 image. Focused tests cover lazy legality,
subscriber order, creation failure, pressure wrapping and distance boundaries.
