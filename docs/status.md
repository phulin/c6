# Implementation status

**The complete headless state/action/turn engine is not implemented.** The public
library contains reusable numerical, state, codec and rules components. It is
not yet usable for complete-game AI evaluation.

| Boundary | Available components | Remaining limits |
| --- | --- | --- |
| Numerical functions | Explicit RNG records/streams, coordinate helpers, bounded combat/movement/yield arithmetic | Complete gameplay RNG ownership and integration, combat execution and movement actions |
| Typed wire data | Map, Player, Unit/Formation, City/District and other supported component schemas; bounded Core prefixes | Unsupported versions/branches, complete import, runtime fixups and unsaved state |
| Owned state | Ordered World records, lookups, retained owners and bounded initialization/query APIs | Complete coupled owner updates and live-world reconstruction |
| Rules/registries | Immutable registered SQLite profiles, explicit row/hash/index identities and ordered definitions | Complete rules providers, arbitrary configuration/content combinations and gameplay integration |
| Action components | Bounded legality, queue/setter/reporting and notification phases with explicit dependencies | Complete callbacks, aftermath, script effects, affected state/RNG and action acceptance |
| Public `Engine` / C ABI | Small snapshot envelope, RNG actions, coordinate helpers and transport | General Game actions, EndTurn and complete World API remain unsupported |
| Turn progression | Reusable internal components | Complete upkeep, every-player decision boundaries and full turn execution |

Supported wire import can represent complete saved Player rosters for particular
admitted schemas. This does not reproduce every initialization side effect,
validate all runtime authority or restore complete gameplay state. Separately
owned later components do not establish complete continuous Core import.

The qualified Linux loaded-state SkipTurn path uses
`c6_data::loaded_actions::transition::prepare_with_auto_notification_queries`
then `PreparedSkipTurn::commit`; successful `AppliedSkipTurn.boundary` reaches
`HeadlessMessagesProcessed` with HOLD and an empty queue. Its typed current
state, rules and profile inputs do not require native per-step answers. One
native loaded initial fixture comparison matches the affected actor/Formation,
actual actor report prefix, serialized complement and captured runtime subset;
complete gameplay state and RNG parity remain unproven.

This finite headless projection excludes source-proven presentation work.
Unknown callbacks or queries return `Unsupported` without mutation; modeled
native rejection can commit its known prefix. Other actions and EndTurn remain
unsupported through the general gameplay API.

The small `Engine::apply` rejects unsupported gameplay actions without mutation.
Some lower-level diagnostic phases preserve completed work on provider errors;
callers must inspect the specific API contract rather than assume rollback or
treat a partial trace as a successful action.

Synthetic tests and source/build/interface checks establish only their declared
boundaries. Numerical parity for one function or records for one profile do not
establish a complete subsystem, full RNG inventory or a legal complete action.
Unknown state remains required until its gameplay role is resolved; it cannot be
filled with defaults or excluded by a class name.

Completion still requires full state and initialization, exact action validation
and mandatory effects, deterministic branching, and complete rules/upkeep until
the next externally supplied decision. Every player decision belongs to the
external policy; the simulator must not silently make native AI choices.

See [architecture/API](architecture.md), [rules inputs](rules-data.md),
[target and decision policy](targets.md) and [performance scope](performance.md).
