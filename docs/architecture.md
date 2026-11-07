# Library architecture and API

The public workspace contains three crates:

| Crate | Responsibility |
| --- | --- |
| `c6-core` | Deterministic numerical functions, typed wire records, owned state and bounded gameplay components |
| `c6-data` | Immutable SQLite rules, definition registries and rules-backed providers; depends on `c6-core` |
| `c6-ffi` | Versioned C shared-library interface; depends on `c6-core` |

The workspace has no source/build dependency on investigative tools, native
harnesses or private captures. Its current components do not compose into a
complete playable simulator; [status](status.md) defines the remaining boundary.

## Supported gameplay runtime construction

Use `c6_data::loaded_actions::session::runtime::Builder` with an explicitly
initialized `LoadedState` and matching typed rules/services. Its `Inputs` moves
the script/RNG and player services once and binds the shared research query,
visibility and unit-update definitions. `factory` joins one canonical bundle of
modifier, requirement, combat collection and factory collection owners to unit
lifecycle callbacks and the same research definitions. `runtime::combat` composes
qualified damage, attacks and formation reports with attack completion callbacks
on that existing bundle. `finish` joins notification inputs, update/research
owners and visibility history into one persistent `Session`.

Keep immutable definitions and live shared query providers in the embedding
arena for the Session lifetime. Rust borrows enforce their lifetime; construction
uses no leaked allocations, self references, second mutable world or per-request
reset. Rejected common construction and factory joins return supplied owners.
Combat report assembly can retain a report-binding prefix on error and returns
its attack inputs. Session failures retain the existing partial gameplay state
and pending decisions. Inspect errors before issuing another request.

The importer supplies typed initialized state, callback registrations and scope
qualifications. It also supplies feature inputs for sight/resource discovery,
city queries, diplomacy and round owners through the existing typed stock
bindings. It does not implement the stock gameplay callback dispatch.
`session::stock_turn_services::StockTurnServices` composes supported upkeep and
control from typed inputs; retain that same service value across `advance` calls.
All policy choices remain external. Missing services and unsupported reached
branches remain errors. This is bounded runtime assembly, not a complete game
initializer, arbitrary save loader or general legal-action enumeration. The
[embedding example](../crates/c6-data/examples/early_game_session.rs) shows typed
assembly followed by requests against the same Session; native capture decoding
is deliberately absent from the public workspace.

## Two state boundaries

`Engine` owns a small `Snapshot` with explicit identities, caller-identified RNG
streams and opaque future sections. `Engine::apply` handles the implemented RNG
operations. General Game actions and EndTurn return typed errors without changing
this envelope. A snapshot identity records a declaration, not proof of a complete
imported gameplay context. Opaque sections preserve bytes without defining how
they evolve.

The larger typed codecs, `world::World` and `loaded_state::LoadedState` provide
separate state and query building blocks. They preserve record order, logical
identity and explicit missing inputs. Decoding fields and constructing lookup
indices do not perform every runtime initialization step, couple all owner
updates, or implement a complete action. Caller-supplied definitions and runtime
bindings must meet each API's declared consistency checks and lifecycle rules.

Low-level diagnostic phases can retain completed mutations when a later provider
fails. Their documented errors and partial traces are not transactions or
successful actions. This differs from the small `Engine::apply` unsupported-action
guarantee; callers must use the contract of the specific API they invoke.

## Numerical and wire contracts

RNG instances carry seed, calls, resets, lock count and synchronous state
explicitly; stream IDs are caller-assigned, with zero reserved for the default.
No host PRNG substitutes for these streams. Registering several streams does not
establish a complete game's RNG inventory. Arithmetic and codecs retain their
explicit widths, wrapping behavior, ordering, version and resource limits.

The [codec module](../crates/c6-core/src/codec.rs) transports the simulator snapshot
envelope. That envelope is distinct from supported component wire records and
from a Civ VI save container. Retaining or reencoding a record establishes no
unimplemented runtime effects. See [rules inputs](rules-data.md) for separate
profile and definition identity checks.

## C interface

[include/c6.h](../include/c6.h) defines ABI v1, numeric action kinds, result codes
and handle ownership. The ABI uses the platform's C calling convention, pointer
width and `size_t`; it is not a portable network ABI. Each handle requires
exclusive external synchronization and must be destroyed before unloading its
creating library. Invalid pointers, double frees and aliasing violate the
contract.

Failed calls preserve outputs as specified by the header; export may update the
required byte count. A draw bound must fit `u16`; the C interface rejects larger
arguments. Snapshot import/export handles the implemented envelope, including
supported legacy upgrades, without claiming whole-game persistence. C EndTurn
is explicitly unsupported.
