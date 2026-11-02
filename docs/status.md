# Implementation status

As of 2026-11-02, c6 provides a tested Rust kernel and loadable C ABI for isolated RNG and geometry operations. It does not yet provide the requested complete headless Civilization VI simulation. No complete player-action sequence or end turn has been reproduced.

| Deliverable | Status and remaining boundary |
| --- | --- |
| Deterministic Rust kernel | Explicit state and typed operations; unsupported game operations return errors |
| State import/export and branching | Canonical versioned envelope, binary/rules identity fields and preserved opaque sections; no Civ VI save decoder |
| RNG | Native arithmetic and logical object fields reproduced; logging/service effects excluded |
| Geometry | Sentinel validity and isolated plot distance reproduced; no initialized map/world |
| Loadable library | Versioned C ABI, checked state round trips and real C caller smoke test |
| Differential infrastructure | Process protocol, identity-pinned corpus, exact observations/RNG traces; isolated original-DLL host |
| Reference/Ghidra | Pinned reference acquisition and private persistent XP2 project; reference mappings retain evidence categories |
| Full gameplay | World, player/entity state, legal actions, combat/economy and turn scheduler unsupported |
| Performance | RNG primitive benchmark available; no game-turn performance measurement |

The native oracle executes methods from the exact XP2 FinalRelease DLL on Linux x86-64 using synthetic objects and documented service stubs. Its success establishes only those methods under that setup. It does not establish that the DLL can run a whole game without its original host.

## Next concrete milestones

1. **Build an initialized native game fixture.** Inventory GameCore host interfaces, global initialization, database/rules inputs and allocation/ownership requirements. Establish reproducible game creation or save loading, one active-player boundary and complete identity metadata. This is the critical prerequisite for meaningful action and turn comparisons; the current PE mapper does not supply it.
2. **Audit a complete state boundary.** Inventory world/map, players, units/cities, component containers, stable IDs, pending events, turn scheduler, caches with semantic effects and every random stream. Determine canonical serialization and restore behavior. Opaque sections preserve data but cannot substitute for this audit.
3. **Implement one real legal action end to end.** Select a small operation from the verified dispatcher, recover its parameters and legality conditions, execute it in the initialized native fixture, and compare all affected state and RNG before adding Rust support.
4. **Reproduce a minimal complete turn.** Establish the actual completion message sequence, iteration/event order, delayed updates and all random streams for a constrained fixture. Compare state after each meaningful boundary and after turn completion. Processor method names alone are insufficient evidence of a complete transition.
5. **Expand configuration coverage and measure useful workloads.** Add rulesets and interacting systems only with pinned fixtures. Measure action and turn throughput once those paths exist; current RNG timing cannot predict it.

## Checks and evidence

The project checks Rust state/geometry/RNG behavior, codec and FFI contracts, harness failure handling, a compiled C consumer and original-DLL primitive comparisons. Run the commands in [README](../README.md) to reproduce the checks with the required local binary. Test counts and timings may change as coverage grows; [architecture](architecture.md), [oracle](oracle.md), [differential protocol](differential.md), [performance](performance.md) and [reference provenance](reference.md) define what each result establishes.

The [engine dependency roadmap](engine-roadmap.md) records the concrete bootstrap services, message-dispatch edges and bounded next investigations. The main unresolved dependency is a reproducible initialized original-game context. Complete state extraction and all gameplay semantics remain substantial implementation work. Neither the recovered symbol map nor the existing primitive corpus closes those gaps.
