# c6

Rust components for an exact, deterministic, headless Civilization VI simulator.
The intended engine accepts externally chosen player decisions and executes the
rules and upkeep between them.

**Complete gameplay state, action execution and turns are not implemented.** The
library currently provides reusable numerical functions, typed wire codecs,
owned state components, rules/registry inputs and bounded action phases. General
`Engine::apply` gameplay requests and EndTurn return unsupported errors. It is
not ready for complete-game AI evaluation. See [current limits](docs/status.md).

A finite Linux loaded-state SkipTurn path is available through
`c6_data::loaded_actions::transition::prepare_with_auto_notification_queries`
and `PreparedSkipTurn::commit`, reaching `HeadlessMessagesProcessed` for its
qualified inputs. It takes typed current state, rules and profile inputs;
complete gameplay state and RNG parity remain unproven.

A qualified Linux initialized-state FoundCity composition also reaches
`HeadlessMessagesProcessed`, including City and District creation, territory,
population, capital and Palace, settler consumption, and an external production
decision. Its representative native comparison matches the captured gameplay
components and RNG endpoints. Source-qualified planning output is omitted;
complete RNG ownership and arbitrary-state/configuration parity remain unproven.
These lower-level Rust APIs require explicit current owners and providers; they
do not enable general `Engine::apply` or C ABI gameplay requests.

For consecutive supported actions, use
`c6_data::loaded_actions::session::Session::from_initialized` with caller-supplied
initialized state, rules and stock services. The [typed early-game example](crates/c6-data/examples/early_game_session.rs)
caches rule definitions, selects research and city production, and advances the
same Session using required turn services. Pending decisions and partial errors
remain visible. The example does not create an initialized game or supply missing
gameplay services; run `cargo run -p c6-data --example early_game_session` for its
input contract.

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
initialized Civ VI game. Complete live-world reconstruction and decision-to-
decision progression remain separate work.

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
