# Implementation status

**The Rust library supports a growing slice of early-game play on one persistent
Session, but the complete headless Civ VI engine is not implemented.** Supported
actions and bounded turn progression go beyond numerical primitives: representative
Linux fixtures exercise city founding, development, research, combat and consecutive
rounds. This is not yet a complete-game AI arena, and there is no meaningful
percentage of the whole game established by the current comparisons.

The gameplay entry point is
[`c6_data::loaded_actions::session::Session`](../crates/c6-data/src/loaded_actions/session.rs).
It retains typed state and callback owners across requests. Callers supply
initialized state, matching rules and required services; `Session::from_initialized`
does not initialize a new Civ VI game. See the
[early-game example](../crates/c6-data/examples/early_game_session.rs) and
[stock turn services](../crates/c6-data/src/loaded_actions/session/stock_turn_services.rs).

| Area | Supported scope | Qualification |
| --- | --- | --- |
| Founding and unit orders | FoundCity; MoveTo, SkipTurn, Fortify and healing | Supported initialized states, movement rules and callbacks; not arbitrary terrain, units or effect branches |
| Development and economy | Farm/Mine construction, ordinary resource-free Farm Repair and forest Chop, and builder charges; city yields and growth; ordinary city tile worker locks; gold purchases of units/buildings and tile purchases | Representative Linux cases; reached rules and callbacks must be supported; citizen unlock/clamp and noncanonical wrapped requests lack separate native cases |
| Production and districts | Natural Scout, Warrior, Monument and Campus completion; Campus placement | Representative Linux cases through supported turn progression and externally supplied production choices; not arbitrary production or district branches |
| Pillage and trade | Enemy city-owned Farm Pillage; domestic trade-route initiation and first physical step | Representative Linux cases; not district/building pillage or complete trade-route lifecycle coverage |
| Research and government | Technology/civic selection and progress; explicit government and policy choices | Bounded definitions, effects and decision paths |
| Religion | Human and bounded nonhuman major Initiation Rites pantheon choice and founding | Representative initialized Linux cases, including stored modifiers, owned-city propagation, reached callbacks and NPC continuation on the same controller; other belief effects remain unsupported |
| Combat and unit progression | Ranged attacks; unit melee with surviving and killed defenders; city-center melee without walls, including bounded noncapital capture and explicit [Keep City](linux-city-keep.md) and [Raze City](linux-city-raze.md); promotions and upgrades | Representative Linux cases through the persistent Session; Return/Liberate, other combat types, modifiers and aftermath branches remain outside the qualified profiles |
| Diplomacy | Direct surprise-war declaration | Bounded cityless case; not complete diplomacy or war consequences in arbitrary worlds |
| Turns and decisions | FoundCity → explicit research/Monument choices → Fortify → three ordinary rounds on one Session; bounded major NPC activation → external Pantheon choice → deactivation to the next player's decision | Compared typed gameplay state, core processors, global religion, represented requirements and known RNG match; raw AI/control differences and incomplete RNG/runtime coverage remain explicit |
| Supporting components | Explicit RNG records/streams, numerical helpers, typed wire codecs, owned component state and registered SQLite rules profiles | Schema/profile-specific; wire transport is distinct from complete save import or live-world reconstruction |
| Small `Engine` / C ABI | Snapshot transport, RNG actions and coordinate helpers | General gameplay actions and EndTurn remain unsupported here; use the Rust Session for the supported gameplay slice |

These are bounded capabilities, not declarations that each named subsystem is
complete. Representative native comparisons cover their recorded affected gameplay
state, callbacks and tracked RNG. They do not establish every reachable branch,
every RNG stream, arbitrary rules/DLC/mod combinations or unrestricted action
sequences. Historical Windows results do not count as Linux support.

The accepted [Farm Pillage](linux-unit-pillage.md) case completes the request
through `HeadlessMessagesProcessed`, preserving genuine pending player decisions.
The domestic trade first-step and natural Campus turn-completion comparisons are
also bounded accepted cases; they do not establish complete subsystem coverage.

Current [dynamic district adjacency](../crates/c6-data/src/city_turn/district_yields.rs)
feeds the production district and city yield queries used by Session. A positive
Linux query case gives Japan's completed Campus 1 Science adjacency, compared
with 0 from the same plot query without its city's dynamic bonuses; all compared
native components and known RNG owners remain unchanged. Terrain, feature,
improvement, appeal and river selectors have Linux source and synthetic coverage,
but lack separate native cases. This query comparison does not establish complete
district, action, RNG or turn parity.

The accepted [ordinary Farm Repair](linux-builder-repair.md) restores a pillaged,
resource-free Farm through the Session and action aftermath without spending a
Builder charge. The accepted ordinary owned, resource-free forest
[Chop](../crates/c6-data/src/loaded_actions/session_chop.rs) adds 21 production
and reduces Builder charges from 3 to 2 in its native comparison; production
completion and last-charge removal were not exercised natively. These are
bounded action results, not complete builder coverage.

Use [`Session::managed_city_plots` and `Session::manage_city_citizen`](../crates/c6-data/src/loaded_actions/session_citizens.rs)
to query current managed tiles and request the native citizen lock cycle at the
ordinary player-action boundary. The request reuses the Session's current yield,
territory and callback owners, then reallocates citizens deterministically. In the
accepted ordinary tile-lock case, an unworked tile becomes the locked worked tile,
the previous tile loses its worker, and current Gold yield changes from 7 to 5;
the other five standard yields, compared typed state and known RNG match. Repeated
unlock requests, forced-worker capacity/population clamps and noncanonical wrapped
request coordinates are implemented from Linux source but lack separate native
comparisons. This does not establish complete citizen management or RNG coverage.

Use [`Session::available_pantheons` and `Session::request_pantheon`](../crates/c6-data/src/loaded_actions/session_pantheon.rs)
for the supported human choice or pending NPC Pantheon choice. Legal options retain an `effects_supported` flag;
unsupported effects return an explicit error. The accepted Initiation Rites case
spends 25 Faith, stores +50 Faith from camp dispersal and +100 healing from camp
dispersal, and preserves availability, shared requirements and owned-city
propagation. Its reached script, Mysticism inspiration and first-pantheon history
moment (+2 era score) match the original, alongside compared typed state and known
RNG. Other belief effects remain unsupported, and actual camp dispersal bonus
application lacks acceptance; this does not establish complete religion coverage.

A representative nonhuman-major case starts with an owned city and ordinary
Warrior. Upkeep queues the Pantheon request, and Religious activation yields a
typed `Pantheon` choice at the belief-selection boundary. Apply it through the
same `request_pantheon` API, then advance the same Session to resume the controller
without repeating upkeep. The compared continuation applies explicit Warrior
SkipTurn and EndTurn requests, consumes external city production, Religious,
Tactical and Homeland responses, and returns to player 0's decision after seven
updates. All those decisions come from external policies.

This case uses city AI priority 2, empty city/Religious contracts and no pending
city purchase request. Priority changes after threat analysis, positive purchase requests
and existing contracts lack this native qualification. Compared typed gameplay,
core processors, global religion, represented requirements/modifiers, GameSync
and libc RNG match. Raw AI/control differences remain recorded; 54 Subject
modifiers lack complete effect/runtime decoding, and the complete RNG inventory
is unfinished. This is bounded continuation support, not complete action or turn
coverage.

The accepted [city-center melee](linux-city-melee.md) case attacks a noncapital
city with no walls and both combatants surviving: 18 attacker damage, 50 city
damage and 3 XP. It completes genuine action cleanup and preserves the compared
gameplay effects and ordered GameSync draws. A source-qualified native AI
targeting tally is omitted and represented by a typed planning observation;
the raw discrepancy and scope are documented. A separate lethal noncapital
capture case completes destruction/reconstruction and melee cleanup with 12
attacker damage, 10 XP and five matching ordered GameSync draws. It yields a typed
pending disposition that survives rewrapping and gates later Session actions.
Disposition execution and a complete gameplay RNG inventory remain outside that
comparison. Separate initialized [Keep](linux-city-keep.md) and
[Raze](linux-city-raze.md) cases resolve the external choice through the same
Session boundary. A further native fixture validates capture followed by explicit
Keep on one persistent Session with the same live owners: no reimport, reload or
initialized-decision injection between actions. The real pending choice gates
ordinary actions, then Keep clears it and restores the native Research blocker.
Compared gameplay state, reached callbacks and known RNG match across the whole
sequence; arbitrary disposition chaining remains unqualified.

The main remaining gap is generality and integration across a whole game: complete
initialized gameplay state, all reached action effects and upkeep, full gameplay
RNG ownership, and long sequences of decisions across supported content. Complete
live initialization and arbitrary Civ VI save loading are unavailable. Save import
is optional fixture functionality rather than a prerequisite for action delivery.
The standalone Rust production library does not run the original game engine.

Every player decision belongs to the external policy. Session exposes pending
semantic decisions and resumes supported upkeep after an explicit choice; it must
not silently choose native AI policy or end a player's turn. Current decision
exposure is not a complete legal-action enumeration for the whole game.

Unsupported branches remain explicit. Some lower-level and Session errors retain
the completed transition prefix and prevent further actions; inspect the API's
failure and pending-decision state instead of treating a partial trace as success
or assuming rollback. The small `Engine::apply` rejects unsupported gameplay
requests without mutation.

Synthetic tests and public build/interface checks establish only their declared
boundaries. Public builds require no private repository or game installation;
matching rules inputs are supplied separately. See [architecture/API](architecture.md),
[rules inputs](rules-data.md), [target and decision policy](targets.md) and
[performance scope](performance.md).
