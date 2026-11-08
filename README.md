# c6

Rust components for an exact, deterministic, headless Civilization VI simulator.
The intended engine accepts externally chosen player decisions and executes the
rules and upkeep between them.

**A bounded early-game gameplay slice is implemented; the complete headless engine
is not.** The Rust Session supports consecutive actions including founding,
movement, improvements, ordinary Farm Repair and forest Chop, production,
research, citizen tile locks, Initiation Rites pantheon choice, purchases
and representative combat, including
nonlethal city-center melee without walls, plus bounded turn progression. A Linux
comparison now follows
one Session from pre-founding state through research and production choices,
Fortify, and three ordinary rounds with explicit decisions for every player.
A [Slinger kill profile](docs/linux-slinger-archery.md) retains its positive Archery
boost and an ordinary round through the next external decision.
A [produced Builder expansion profile](docs/linux-produced-builder-expansion.md)
joins queued Settler and Builder completion, second founding, external travel,
a Farm and the following round on one Session. The [resource Farm profile](docs/linux-resource-farm-irrigation.md)
retains its Irrigation boost and resource acquisition; the [Plantation profile](docs/linux-plantation-luxury.md)
adds a newly available luxury, both cities' amenities and following upkeep. The
[revealed Iron Mine profile](docs/linux-iron-mine-stockpile.md) retains both construction
boosts and strategic-resource extraction through ordinary upkeep.
A [Scout natural-wonder profile](docs/linux-natural-wonder-discovery.md) preserves
positive sight experience, Astrology, History and era effects through the next
ordinary round on the same Session. A [Scout Alpine profile](docs/linux-scout-alpine.md)
extends discovery through an explicitly selected promotion, ordinary upkeep and
hill movement that demonstrates the earned effect. The [tribal-village profile](docs/linux-tribal-village-reward.md)
continues that Scout into an original village, retains its randomly selected
experience reward and completes the following ordinary round on the same Session.
The [random research-boost profile](docs/linux-village-research-boost.md) covers
original technology and civic rewards, unchanged research queues and their following
rounds. These sequences also reach a surviving flood-damaged Scout and a targetless
drought attempt. The [flood-expiry continuation](docs/linux-flood-expiry.md) advances
the technology case through one more ordinary round on the same Session, preserving
flood history and the surviving unit through turn 6.
A separate bounded Linux case covers nonhuman major activation with a city and
Warrior, an external Pantheon choice, and deactivation to the next player's decision.
Coverage remains
specific to supported states, rules and effects; it is not ready for complete-game
AI evaluation.
See [current coverage and limits](docs/status.md).

Use `c6_data::loaded_actions::session::Session::from_initialized` with
caller-supplied initialized typed state, matching rules and required services.
The [typed early-game example](crates/c6-data/examples/early_game_session.rs)
caches rule definitions, selects research and city production, and advances the
same Session. See the [full retained sequence and decision contract](docs/linux-found-city.md).
Pending decisions and partial errors remain visible. The example
does not create an initialized game; run
`cargo run -p c6-data --example early_game_session` for its input contract.
General `Engine::apply` and C ABI gameplay actions, including EndTurn, remain
unsupported. Complete gameplay state and RNG ownership, arbitrary configurations
and long-game integration remain unfinished.

The active compatibility target is the supplied Linux XP2 profile. Historical
Windows data does not establish Linux coverage. Supported operations retain
explicit versions, input identities and missing-provider errors; unknown bytes
cannot be advanced as gameplay state. See [target and decision policy](docs/targets.md).

## Build and check

With Rust/Cargo and a C11 compiler installed, run from this checkout:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p c6-ffi
bash tools/check/c_abi_smoke.sh
```

The workspace contains `c6-core`, `c6-data` and `c6-ffi`. Public builds and
synthetic checks require no private repository or game installation. Loading a
registered rules profile separately requires matching caller-supplied SQLite
inputs. These checks establish implementation/API behavior within their test
scope, not complete gameplay equivalence.

The Linux shared library is `target/release/libc6_ffi.so`; its public contract is
[include/c6.h](include/c6.h). The C interface currently exposes numerical RNG,
coordinate helpers and simulator snapshot transport, not a complete World API.
See [architecture and API](docs/architecture.md), [rules inputs](docs/rules-data.md)
and [performance scope](docs/performance.md).

## Rust API example

```rust
use c6_core::{rng::RandomState, Action, Engine, Outcome, Snapshot};

fn main() -> Result<(), c6_core::Error> {
    let mut engine = Engine::new(Snapshot::new(RandomState::new(37)))?;
    let value = engine.apply(&Action::RandomDraw { bound: 100 })?;
    assert!(matches!(value, Outcome::Random(n) if n < 100));

    let before = engine.snapshot().clone();
    assert!(engine.apply(&Action::EndTurn).is_err());
    assert_eq!(engine.snapshot(), &before);
    Ok(())
}
```

This operates on the small `Engine` envelope. Cloning it does not clone a fully
initialized Civ VI game. Complete live-world reconstruction remains unfinished;
supported decision-to-decision progression uses the Rust Session described above.

## Attribution

Workspace packages declare the MIT license. Reference-derived names and
explanations acknowledge cru121, *Civ6-GameCore-reference*, revision
`5af389445118bed2bdebf7a8a42b219c8fba5175`:
[reference repository](https://github.com/cru121/civ6-gamecore-reference),
[documentation/data license CC BY 4.0](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/LICENSE),
[tools license MIT](https://github.com/cru121/civ6-gamecore-reference/blob/5af389445118bed2bdebf7a8a42b219c8fba5175/LICENSE-CODE).
Descriptions are adapted to the Rust interfaces and bounded Linux contracts.
Upstream credits Sukrit Tan, ChimpanG, WildW and Community Extension contributors.
Reference names alone do not establish behavior. Game binaries, assets and saves
are not distributed with this public library.
