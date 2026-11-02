# Implementation status

The supplied Linux XP2 profile is the sole execution and acceptance target for the complete headless state/action/turn library. Windows work is frozen historical evidence. The [public-depot audit](version-audit.md) matches the installed Linux depot metadata to Valve's public branch; [target policy](targets.md) records exact binary identities and validation boundaries.

The full goal remains incomplete. Native Linux context initialization and cleanup, original Base English localization, genuine Lua/script providers and original StartupGame return have executed successfully, followed by original context/provider cleanup and handle closure. Original saved-configuration restoration returns and produces deterministic canonical output with explicit differences. Actual Linux base rules exports now match Rust and independent Python validation; full XP2 activation and original game loading remain active work. No Linux gameplay save has been restored, no legal action has passed complete affected-state comparison, and no complete turn has been reproduced. Follow [Linux runtime evidence](linux-runtime.md) for subsequent checkpoints.

| Deliverable | Linux target status |
| --- | --- |
| Native loader/context | Genuine host exports, GameCore loading, Initialize flag transition, context destruction, host-service cleanup and handle closure verified |
| Original localization | Gated Base English sources, native dictionary initialization/lookup and shutdown verified |
| Script VM and StartupGame | Genuine Lua provider and original StartupGame return/cleanup verified; no restored playable session |
| Saved configuration | Native reader returns with deterministic canonical output; one added key, three flag changes and pending user settings remain explicit |
| Stream/Load/Save adapters | Genuine native stream checks pass; populated-base Load probe times out, no successful gameplay round trip |
| Rules databases | Two independent Linux base exports, 396 tables / 25,246 rows; exact file/schema/typed-row parity in Rust and Python, tamper refusals verified; full XP2 activation pending |
| Rust kernel and C ABI | State envelope, explicit identities, branching and isolated operations exist; historical Windows tests do not certify Linux mechanics |
| Complete game state | World, players/entities, pending events, caches and all RNG ownership not yet represented/audited |
| Legal actions and turns | Unsupported |

Linux native checks apply to their exact module/configuration identities. Context initialization is distinct from configured game startup. Original application main is bypassed; normal App startup, Steam/network initialization and application shutdown have not been reproduced. The runtime uses a bounded native host path with genuine providers rather than substituting gameplay behavior. Full XP2 activation is not supplied by the populated base recipe: the original modding framework owns content activation and context bootstrap/reload, which is still being integrated.

## Current data and restore boundary

The Linux base fixture contains 78 configuration tables / 1,280 rows and 318 gameplay tables / 23,966 rows. Two native exports are identical, and Rust plus independent Python match the complete schema, typed-row and file fingerprints. Mutation, row-ID, schema and physical-file changes are refused. This establishes immutable base rules input; SQLite row identities are not GameCore definition IDs, and unit base values are not complete mechanics. See [Linux rules](linux-rules.md) and [Rust rules input](rules-data.md).

The original Linux configuration reader returns successfully, retains the saved ruleset/mod identifiers and produces deterministic native output. It adds one key, changes three flags, and leaves user settings pending; equality to the saved bytes or active full-session configuration is not claimed. The populated-base Load probe exceeds its process time bound without a successful Load result. An earlier empty-rules/GameSpeed failure is preserved separately. See [saved configuration](linux-saved-configuration.md) and [runtime evidence](linux-runtime.md).

## Historical coverage boundary

Preserved Windows work includes RNG and coordinate primitives, normal RNG serialization, a null-parameter unit-operation body codec, native context/startup probes and repeatable rules SQLite exports. The historical `c6-data` profile matches those Windows-native exports. Linux base data parity is established separately above; Windows work establishes no Linux operation parity, legal action, restored save or complete turn. Windows execution and further Windows validation are inactive.

The `C6SN` envelope is custom simulator transport, not Civ VI save import. Opaque section retention does not establish semantics. A message body codec is not action legality or execution. A typed rules row is not a GameCore definition ID or an implemented mechanic.

## Next concrete milestones

1. **Activate the complete Linux XP2 fixture.** Integrate original modding-framework ownership of context bootstrap/reload and authentic content activation, then validate rules/DLC/mod identities and startup ordering. Preserve configuration canonical differences and pending-state semantics rather than silently accepting them as exact restoration.
2. **Restore a pinned native Linux fixture.** Extend the proven base export to an authentic activated XP2 profile; resolve the current Load timeout with original services and record return/consumption and complete restored state. Validate native Save/Load before treating retained bytes as a reusable snapshot.
3. **Audit and port full state.** Inventory world/map, entities and stable IDs, component containers, queued events, turn scheduler, semantic caches and every random stream. Establish canonical state extraction/restoration and branching.
4. **Validate Linux mechanics and one legal action.** Recheck reused primitives against the Linux target, then execute a real legal action in an initialized fixture and compare all outputs, state mutations, events and RNG before adding Rust support.
5. **Reproduce a complete turn.** Match actual completion messages, phases, player/event ordering, delayed updates and every affected subsystem. Measure action/turn throughput only after these paths exist.

## Checks and evidence

The [README](../README.md) contains active Rust/FFI and Linux context commands. [Linux runtime](linux-runtime.md), [host services](linux-host-services.md), [asset identity](linux-assets.md) and [version audit](version-audit.md) define the native evidence and limits. [Rules data](rules-data.md) now separates the validated Linux base profile from its historical Windows profile. [Oracle](oracle.md) and Windows bootstrap documents describe frozen historical execution; their results cannot be transferred to Linux without target-specific comparisons. Rules load/lookup measurements cover data only; they do not predict full-turn throughput. Full gameplay equivalence and useful game-turn performance remain outstanding.
