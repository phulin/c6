# Implementation status

The supplied Linux XP2 profile is the sole execution and acceptance target for the complete headless state/action/turn library. Windows work is frozen historical evidence. The [public-depot audit](version-audit.md) matches the installed Linux depot metadata to Valve's public branch; [target policy](targets.md) records exact binary identities and validation boundaries.

The full goal remains incomplete. Native Linux context initialization and cleanup, original Base English localization, genuine Lua/script providers and original StartupGame return have executed successfully, followed by original context/provider cleanup and handle closure. Saved-configuration restoration and original game loading are active work. No Linux gameplay save has been restored, no legal action has passed complete affected-state comparison, and no complete turn has been reproduced. Follow [Linux runtime evidence](linux-runtime.md) for subsequent checkpoints.

| Deliverable | Linux target status |
| --- | --- |
| Native loader/context | Genuine host exports, GameCore loading, Initialize flag transition, context destruction, host-service cleanup and handle closure verified |
| Original localization | Gated Base English sources, native dictionary initialization/lookup and shutdown verified |
| Script VM and StartupGame | Genuine Lua provider and original StartupGame return/cleanup verified; no restored playable session |
| Saved configuration | Original native configuration restore being reconstructed; no accepted full fixture |
| Stream/Load/Save adapters | Genuine native stream boundary checks and typed adapters being integrated; gameplay round trip unsupported |
| Rules databases | Linux-native export and complete Rust parity pending |
| Rust kernel and C ABI | State envelope, explicit identities, branching and isolated operations exist; historical Windows tests do not certify Linux mechanics |
| Complete game state | World, players/entities, pending events, caches and all RNG ownership not yet represented/audited |
| Legal actions and turns | Unsupported |

Linux native checks apply to their exact module/configuration identities. Context initialization is distinct from configured game startup. Original application main is bypassed; normal App startup, Steam/network initialization and application shutdown have not been reproduced. The runtime uses a bounded native host path with genuine providers rather than substituting gameplay behavior.

## Historical coverage boundary

Preserved Windows work includes RNG and coordinate primitives, normal RNG serialization, a null-parameter unit-operation body codec, native context/startup probes and repeatable rules SQLite exports. The Rust `c6-data` loader matches those Windows-native exports. None of this establishes Linux rules identity, Linux operation parity, a legal action, a restored save or a complete turn. Windows execution and further Windows validation are inactive.

The `C6SN` envelope is custom simulator transport, not Civ VI save import. Opaque section retention does not establish semantics. A message body codec is not action legality or execution. A typed rules row is not a GameCore definition ID or an implemented mechanic.

## Next concrete milestones

1. **Complete the configured Linux fixture.** Preserve successful original startup/cleanup, restore saved configuration with the Linux-native ABI, and validate rules/DLC/mod identities and startup ordering. Record every unsupported dependency explicitly.
2. **Restore a pinned native Linux fixture.** Export and fingerprint actual Linux rules inputs; enter original Load using genuine streams, record return/consumption and compare complete restored state. Validate native Save/Load before treating retained bytes as a reusable snapshot.
3. **Audit and port full state.** Inventory world/map, entities and stable IDs, component containers, queued events, turn scheduler, semantic caches and every random stream. Establish canonical state extraction/restoration and branching.
4. **Validate Linux mechanics and one legal action.** Recheck reused primitives against the Linux target, then execute a real legal action in an initialized fixture and compare all outputs, state mutations, events and RNG before adding Rust support.
5. **Reproduce a complete turn.** Match actual completion messages, phases, player/event ordering, delayed updates and every affected subsystem. Measure action/turn throughput only after these paths exist.

## Checks and evidence

The [README](../README.md) contains active Rust/FFI and Linux context commands. [Linux runtime](linux-runtime.md), [host services](linux-host-services.md), [asset identity](linux-assets.md) and [version audit](version-audit.md) define the native evidence and limits. Existing [rules-data](rules-data.md), [oracle](oracle.md) and Windows bootstrap documents describe historical profiles; their results cannot be transferred to Linux without target-specific comparisons. Full gameplay equivalence and useful game-turn performance remain outstanding.
