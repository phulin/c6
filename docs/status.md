# Implementation status

The supplied Linux XP2 profile is the sole execution and acceptance target for the complete exact headless state/action/turn library. Windows work is frozen historical evidence. [Target policy](targets.md) records module identities and profile-specific validation.

**Native XP2 Core state loading now works under an identified diagnostic profile. The complete Rust simulator remains unimplemented.** Native full-host saved-load finalization, branch-restorable state, legal actions and complete turns are still unverified.

| Deliverable | Verified scope and remaining boundary |
| --- | --- |
| Native host/providers | Original filesystem, TempHeap, App/database/localization/script providers, startup and owned cleanup pass |
| XP2 activation | Original Configure/Update and cleanup pass; one enabled/active mod and 46 components observed in getter order, not importer execution order |
| Rules data | Two XP2 diagnostic exports: 506 tables / 43,565 rows; exact Rust/Python file/schema/typed-row parity and refusal checks; separate base profile retained |
| Offline session | Original Attach/Leave and full cleanup pass for Base and XP2 contexts; session/local peer independently observed |
| Core benchmark Load | Native OK, all 76,556,938 input bytes consumed, successful Shutdown/session/framework/provider cleanup |
| Repeated Core Save | Four outputs across two independent processes are identical, 80,657,462 bytes each |
| Captured-output reload | Native OK and cleanup repeat; resulting Saves differ from input in 89,198 bytes; byte-stable branching fails |
| Read-only loaded game facade | Both input profiles report turn 592, phase 0, local player 0 and false pending/update/processing flags; serialized before/after equality passes |
| Whole-save request preparation | Original RequestLoad retains an owned full-container copy and reset releases it; CheckRequest/Begin and later host finalization unverified |
| Rust primitives | Isolated Linux Random/normal record and stateless coordinate validity/distance match original code |
| Complete Rust state/actions/turns | Unsupported; no initialized native legal-action or complete-turn differential comparison |

## Diagnostic profile and limits

The passing content profile gates eleven native modules and all 2,779 original content inputs. It uses an original constructor-state LoadGameViewState callback with request None and censorship=false, plus a genuinely constructed UI provider without graphics startup. Saved RequestLoad/Begin semantics are not inferred. Native getters report one enabled mod, one active mod and 46 components; their container order is not an execution trace. See [activation](linux-mod-activation.md), [rules input](rules-data.md) and [independent XP2 rules review](linux-xp2-rules-review.md).

Original session attachment precedes lower Core Load, supplying the genuinely observed local peer. This lower path is distinct from the complete host container/request workflow. Original CheckRequest/Begin, PostInitialize, host caches, active-user selection, pending settings and full saved-profile equivalence remain acceptance gates. [Save lifecycle](linux-save-lifecycle.md) and [action boundaries](linux-actions.md) retain the exact ordering/ownership contracts.

Benchmark-input Load and repeated original Saves reproduce across independent processes. Captured-output reloading also succeeds, but produces a different repeatable serialized output. The 89,198 changed bytes have unproven semantics and cause; they must not be normalized away or assigned to AI state by assumption. Successful Results, full input consumption and stable consecutive Saves do not establish a branch-restorable complete state. [Core Load/Save evidence](linux-game-load.md) records the two profiles and difference report.

The native read-only facade returns turn 592, phase 0 and Context/Game local player 0 in both profiles; pending messages, NeedsUpdate, NeedsTurnAdvance, IsAiProcessing and IsTurnProcessing are false. Wrapper release and these queries leave before/after Core Saves identical. These coarse observations do not imply action readiness, all AI state equivalence or complete live-state identity. Queues, caches, events, jobs and host/session state require separate coverage.

## Rust and tooling boundary

`c6-data` verifies registered base and activated-XP2 diagnostic SQLite profiles, retaining complete typed rows and metadata. SQLite row IDs are not established engine definition IDs; data loading and unit projections are not mechanics. Its timings measure data operations only. The `C6SN` envelope is simulator transport, not Civ VI save import; opaque retention cannot advance unknown state.

[Linux Random](linux-random.md) covers isolated arithmetic/state, independent objects and normal complete records; gameplay stream ownership/lifecycle, logging and legacy/failing streams remain excluded. [Linux geometry](linux-geometry.md) covers sentinel validity and stateless distance, not initialized maps, movement or pathfinding. Historical Windows evidence establishes no additional Linux coverage.

The private persistent `c6-linux.gpr` project has targeted decompiles, guarded MCP access and checksummed snapshots, including current canonical-stream contracts. This remains targeted analysis rather than completed whole-program/DWARF analysis; see [Ghidra tools](ghidra.md).

## Next concrete milestones

1. **Complete the original saved-game host workflow.** Integrate RequestLoad checking/Begin, preserved callback policy, session/container stages, PostInitialize, caches and original active-user handling with complete cleanup and profile identities.
2. **Establish complete restorable state.** Explain the reproducible Core roundtrip differences without assumptions, audit all serialized and live/transient state, and prove deterministic branching under a defined full-state comparison.
3. **Execute a real legal action.** Use actual loaded units/players, complete legality predicates and synchronized host processing; compare all state, events and RNG effects before implementing matching Rust mechanics.
4. **Port complete state and systems to Rust.** Preserve IDs, containers, iteration/event order, pending work and every random stream; retain Linux-native differential fixtures for each supported operation.
5. **Reproduce a complete turn.** Match genuine completion messages, phases, player/control transitions and every affected subsystem. Benchmark full workloads only once those paths exist.

The [README](../README.md) links active commands. [Native ABI review](linux-native-abi-review.md), [XP2 rules review](linux-xp2-rules-review.md), [runtime evidence](linux-runtime.md) and the owner documents above define what each checkpoint establishes. Full gameplay equivalence remains outstanding.
