# Linux FoundCity and a persistent early-game sequence

`c6_core::found_city` implements the pinned Linux handler's ordered `CanStart`
and `Start` logic. `found_city::placement` computes the stock legal-location
checks and current-map minimum-distance scans. `c6_data::found_city` supplies
admitted `Units.FoundCity`, `Features.Removable` and `Features.Settlement` values
and reads the existing canonical `Player.Units.no_settling_cities` flag.

`c6_data::loaded_actions::session::Session::found_city` composes the supported
stock request, manager/script legality, City construction, placement, naming,
capital/Palace effects, CityCreated callbacks and delayed Settler consumption.
These remain bounded Linux paths: missing effects fail explicitly, and callers
must supply the actual initialized state, matching rules and callback services.
The standalone library does not create a new initialized game or require the
original engine at runtime.

Construct `Session::from_initialized` once. Keep its World, Script and RNG,
research, visibility history, modifier/requirement stores, quests and shared
territory alive across every action and turn. Cache immutable rule definitions
once. Newly founded cities must become visible to later current-city queries and
research traversal through those same owners; replacing a runtime from an AFTER
snapshot would skip the transition being simulated.

The compared ordinary-land, singleton-formation profile executes this sequence:

1. Found the initial Settler's city with `found_city`.
2. Supply explicit `request_technology` / `request_civic` choices when needed and
   select Monument with `choose_building`; production starts without seeded progress.
3. Issue the existing Warrior's `fortify` order.
4. Advance three rounds with `advance(Request::EndTurn { .. })` and the retained
   `StockTurnServices`. Supply each reached research or ordinary action decision,
   then explicitly end that player's turn. Preserve the queued Fortify order.
5. For an active NPC's recorded SkipTurn message, use `skip_turn_message` at its
   suspended `PlayerActions` boundary. This runs UnitOperation.Execute and the
   same manager/diplomacy/notification aftermath. The ordinary `skip_turn` request
   retains its earlier local-player Broadcast guard.

`turn_loop::Outcome::Choice` exposes a genuine external decision;
`DecisionReady` returns control at a stable human boundary. Neither authorizes
native AI selection. Source-positioned `npc_unit_decisions::Provider` callbacks
also require an explicit policy response. This fixture supplies legal SkipTurn
orders and DEFER responses at Tactical/Homeland selection calls. It deliberately
omits policy recruitment, selection and their policy RNG, while retaining ordinary
DoTurnEnd controller cleanup, action effects and nonplanning upkeep. This is not
a general replacement for arbitrary AI planners or multiunit formations.

After three rounds, the same city's food is 1,536 Q8, Monument progress is 3,072
Q8, technology progress is 1,920 Q8 and civic progress is 996 Q8. Compared gameplay
fields, global religion, core processors, map/visibility, rosters, represented
requirements, GameSync and matched libc RNG agree with the original Linux run.
Raw property annotations and requirement reference differences remain visible;
the complete RNG inventory and requirement runtime coverage are not closed. This
bounded result does not establish complete Civ VI actions or arbitrary turns.

Failures retain any completed gameplay/RNG prefix. Inspect `failure()`,
`pending_choice()` and the returned error; do not treat partial state as success
or assume rollback. The [typed embedding example](../crates/c6-data/examples/early_game_session.rs)
shows initialization and retained research/production/turn services; it requires
caller-supplied initialized inputs.

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
