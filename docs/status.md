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
| Development and economy | Farm/Mine construction and builder charges; city yields and growth; gold purchases of units/buildings and tile purchases | Representative Linux cases; reached rules and callbacks must be supported |
| Production and districts | Natural Scout, Warrior and Monument completion; Campus placement and completion through explicit production progress | Natural Campus completion through turn progression is under validation |
| Research and government | Technology/civic selection and progress; explicit government and policy choices | Bounded definitions, effects and decision paths |
| Combat and unit progression | Ranged attacks; melee with surviving and killed defenders; promotions and upgrades | Representative Linux cases, including persistent Session melee; not all combat types, modifiers or aftermath branches |
| Diplomacy | Direct surprise-war declaration | Bounded cityless case; not complete diplomacy or war consequences in arbitrary worlds |
| Turns and decisions | EndTurn/resume on the same Session with stock services; one early-game full round and a two-round sequence compared against Linux captures | Captured gameplay state and tracked RNG match within those cases; not arbitrary long-game turn equivalence or a complete RNG inventory |
| Supporting components | Explicit RNG records/streams, numerical helpers, typed wire codecs, owned component state and registered SQLite rules profiles | Schema/profile-specific; wire transport is distinct from complete save import or live-world reconstruction |
| Small `Engine` / C ABI | Snapshot transport, RNG actions and coordinate helpers | General gameplay actions and EndTurn remain unsupported here; use the Rust Session for the supported gameplay slice |

These are bounded capabilities, not declarations that each named subsystem is
complete. Representative native comparisons cover their recorded affected gameplay
state, callbacks and tracked RNG. They do not establish every reachable branch,
every RNG stream, arbitrary rules/DLC/mod combinations or unrestricted action
sequences. Historical Windows results do not count as Linux support.

Pillage, physical trade-route movement and natural Campus turn completion are
active work, not completed native-validated coverage in the table above.

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
