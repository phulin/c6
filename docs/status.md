# Implementation status

As of 2026-11-02, c6 provides a tested Rust kernel for isolated RNG, geometry and action-message transport, a loadable C ABI, and an original Windows DLL bootstrap running under Wine. Genuine native contexts initialize with original database fixtures and complete teardown. It does not yet provide the requested complete headless Civilization VI simulation. No complete player-action sequence or end turn has been reproduced.

| Deliverable | Status and remaining boundary |
| --- | --- |
| Deterministic Rust kernel | Explicit state and typed operations; unsupported game operations return errors |
| State import/export and branching | Canonical versioned envelope, binary/rules identity fields and preserved opaque sections; no Civ VI save decoder |
| RNG | Native arithmetic and logical object fields reproduced; logging/service effects excluded |
| Native RNG serialization | Original 18-byte normal `Random::Write`/`Read` format; raw synchronous-byte records preserved separately from the logical bool; legacy-vector and short-stream behavior excluded |
| Geometry | Sentinel validity and isolated plot distance reproduced; no initialized map/world |
| Loadable library | Versioned C ABI, checked state round trips and real C caller smoke test |
| Action-message transport | Exact null-parameter unit-operation body serializer and strict Rust parser; no manager legality, queue execution or gameplay mutation |
| Differential infrastructure | Process protocol, identity-pinned corpora, exact observations/RNG traces and isolated original-DLL hosts |
| Native Windows bootstrap | Full DLL loader, genuine context factory, Initialize/IsInitialized transition, destruction and unload with real DatabaseDLL fixtures |
| Original content and localization | Original host SQL/XML importer and Make_Hash; declared Base/XP2 content fixtures and measured native embedded-database writes; bounded original en_US localization/ICU fixture |
| Save research | Pinned benchmark metadata and extracted GameCore stream; original Load is gated on successful StartupGame and is not yet established |
| Reference/Ghidra | Pinned reference acquisition and private persistent XP2 project; reference mappings retain evidence categories |
| Full gameplay | World, player/entity state, legal actions, combat/economy and turn scheduler unsupported |
| Performance | RNG primitive benchmark available; no game-turn performance measurement |

Two native execution paths serve different purposes. The isolated mapper executes bounded original methods using synthetic objects and documented stubs; it does not run DLL initialization. The Wine bootstrap loads the whole original DLL normally and invokes genuine context interfaces with explicit host services. The mapped original host executable supplies specific verified code paths; its entry point and global constructors are not run. Neither path establishes complete gameplay parity.

The declared Base and XP2 recipes import original SQL/XML through genuine host and database code. The XP2 fixture also executes the original embedded GameCore database phase and measures its generated type/effect mutations before completing context initialization and teardown. These explicit recipes do not execute the engine’s full mod-selection resolver or prove equivalence to the saved session. Successful source execution or a true InitializeDatabase return alone is insufficient; transaction, row-mutation and foreign-key checks remain part of database acceptance. Original localization initialization and a known English lookup have passed; the latest XP2 StartupGame probe progresses to tuner initialization, then faults because the script service is unavailable. StartupGame has not returned and original Load has not been called. Full DLC/mod localization resolution remains unsupported. See [native runtime](bootstrap-runtime.md), [host services](bootstrap-services.md), [content evidence](assets.md) and [localization](bootstrap-localization.md) for current observations.

The native RNG payload remains distinct from the custom `C6SN` envelope and Civ VI saves. Short streams and nonzero legacy flags are refused before native reads; raw synchronous-byte values above 1 can round-trip in serialized records but cannot be applied to Rust boolean state. Likewise, exact action-message serialization does not validate action legality or affected gameplay state.

## Next concrete milestones

1. **Complete native game startup and load a pinned fixture.** Preserve the successful declared XP2 content and embedded GameCore database phase, then establish the saved session’s live configuration and authentic mod-selection order. Provide a genuine initialized script system/HavokScript VM for the observed tuner-initialization boundary, verify StartupGame returns, then execute the prepared original Load adapter. Preserve saved configuration, DLC/mod ordering, rules identities and complete stream observations. A returned context Initialize flag is insufficient.
2. **Audit a complete state boundary.** Inventory world/map, players, units/cities, component containers, stable IDs, pending events, turn scheduler, caches with semantic effects and every random stream. Determine canonical serialization and restore behavior. Opaque sections preserve data but cannot substitute for this audit.
3. **Implement one real legal action end to end.** Select a small operation from the verified dispatcher, recover its parameters and legality conditions, execute it in the initialized native fixture, and compare all affected state and RNG before adding Rust support.
4. **Reproduce a minimal complete turn.** Establish the actual completion message sequence, iteration/event order, delayed updates and all random streams for a constrained fixture. Compare state after each meaningful boundary and after turn completion. Processor method names alone are insufficient evidence of a complete transition.
5. **Expand configuration coverage and measure useful workloads.** Add rulesets and interacting systems only with pinned fixtures. Measure action and turn throughput once those paths exist; current RNG timing cannot predict it.

## Checks and evidence

The project checks Rust state/geometry/RNG behavior, serialization and message codecs, FFI contracts, harness failure handling, a compiled C consumer and original-DLL comparisons. Separate [bootstrap checks](bootstrap-validation.md) verify original context lifecycle, database diagnostics and identity rejection; later content/startup observations have their own evidence records. Run the commands in [README](../README.md) to reproduce the checks with the required local binary. Test counts and timings may change as coverage grows; [architecture](architecture.md), [oracle](oracle.md), [differential protocol](differential.md), [performance](performance.md) and [reference provenance](reference.md) define what each result establishes.

The [engine dependency roadmap](engine-roadmap.md) records the concrete bootstrap services, message-dispatch edges and bounded next investigations. The main unresolved dependency is progression from the verified native context/data bootstrap to a reproducible initialized gameplay session, including authentic content/configuration, successful StartupGame and save-state restoration. The read-only [Linux installation inventory](engine-roadmap.md#incoming-linux-installation) can identify supplied ELF binaries and their dependencies; the matching `Civ6` host, `libGameCore_XP2.so` and installation data are needed to investigate that separate platform. Complete state extraction and all gameplay semantics remain substantial implementation work. Neither the recovered symbol map nor the existing primitive corpus closes those gaps.
