# Hardware adapter for one genuine full Manager invocation

The new [adapter](../tools/linux/full_manager_observer_runner.py) connects the reviewed [passive Lua observer](linux-script-manager-live.md) to the [separate full Manager caller](linux-full-manager-fixture.md). It implements actual GDB installation, outer-drain scheduling, mapped-code/source/profile gates, native frame attribution and lock provenance. It does not launch a process, create a native profile or execute originals during preparation. Full Manager parity, callback effects and complete gameplay state/actions/RNG/turns remain unaccepted.

The [source audit](../evidence/linux/full-manager-observer-runner-source.json) rechecks original host/Core bodies, compiles all offsets of the actual 88-byte C observation descriptor, and exercises descriptor/frame/plan refusals and the real installer/drain using fake GDB hardware lanes. No synthetic test loads original game code. The caller's independent review remains separate from this adapter's review and the future concrete loader/guard/profile admission.

## Sole-runner contract

The sole runner calls `install(runtime_plan_path, fresh_private_directory)` inside its separately launched GDB driver, then runs `c6-full-manager-drain`. The driver owns exact process/session identity, timeout, terminal native cleanup interpretation and failure disposal. Reuse the existing Military supervisor's exact-PID/start-time/session disposal discipline; never leave a failed or timed-out inferior executing. This adapter leaves a failed inferior stopped for that owner and makes no native teardown, detach or kill call. `exit.json` explicitly records `observer_failed` and does not claim original cleanup.

The driver configures pending breakpoints and all-stop mode before installation. It must disclose its `starti` and GDB loader-rendezvous instrumentation separately. This adapter's marker and original target breakpoints are exclusively `BP_HARDWARE_BREAKPOINT`; it performs no inferior function call, Lua lookup, lock operation, GC step, PC skip, FinishBreakpoint, software game patch or native policy call. Stop callbacks queue only registers/thread identity. All memory/frame analysis and breakpoint deletion/creation run after the stop callback unwinds, in the outer drain.

The pending own marker is installed before `FACADE_ACQUIRED` phase 1. At the first real marker the adapter gates exact mapped Civ6/Core/preload/dependency file identities, profile and source closure, compiled marker/caller code and mandatory original bodies. The source caller reaches this marker after its genuine facade acquisition, before actor/registry/Operation/predicate calls. The separately reviewed runner must also gate the original acquisition/profile before launch; a phase-1 check does not retroactively admit earlier calls.

## Immutable plan and complete native frame evidence

The profile key is `lower_core_full_manager_observer` with `static_plan` and `static_plan_sha256`. The runtime plan adds `runtime_gate`, containing the prepared report path/hash, exact configuration hash, all frozen source files and `static_plan_path`. The adapter rechecks the actual static file hash and its JSON against the configuration; it never derives expected results from native answers.

The static schema is `c6-linux-full-manager-observer-plan-v1`, with profile kind `genuine_lower_core_full_Manager`. It requires canonical `host`, `core` and `preload` paths, a `modules` hash map including executable frame dependencies, complete `live_bodies`, compiled marker RVA, and these reviewed contracts:

- `caller_suffix_frames`: the exact normalized compiled caller return through main/libc/startup root, following the original Manager frame. Each entry has `module` and integer `relative_pc`.
- `window_parent_prefixes`: accepted exact original parent chains through Manager continuation for every natural Lua checkpoint. This includes legitimate protected-VM frames; an unknown collector/reentrant/other parent is rejected rather than inferred idle.
- `marker_parent_frames`: exact complete marker/caller/main-entry frame chains for phases 1 through 9, with alternatives only when individually source-closed.
- `main_entry_witness`: the reviewed preload module and RVA of its own recorded OS main TID field. The future loader will record `observed_mod_main_tid` at original main entry under `C6_FULL_MANAGER_FIXTURE`. The adapter copies that field and matches actual inferior PID/LWP without calling its getter or converting a native lock token.
- `stack_contract_review`: path/hash of a source review packet with schema `c6-linux-full-manager-stack-contract-v1`, the same frame/marker/body/witness contracts and complete reviewed source map. Its `no_VM_or_GC_parent_frames_source_closed` assertion must be backed by those exact source/call-chain contracts; actual complete unwinds must match them.

The future compiled preload's exact marker/return/main-witness addresses and complete original protected-call parent prefixes are still required. No plan containing those identities has been admitted here. Missing contracts reject before attachment/continuation rather than inventing zero Lua/GC parents from controller idle.

Mapped module files must match source hashes, device/inode and stable file metadata. All observed executable frames belong to those pinned mappings. Unwinds are bounded to 128 frames and must match the complete reviewed suffix; truncation, unknown executable owners and reentrant Manager continuations fail. The adapter intrinsically checks the original script backend, HKS decoder layout methods, lifecycle/critical-section implementations **and Platform Enter/Leave trampolines**, complete negative-index function helper, concrete host slots and named strings. Additional compiled preload/marker/caller bodies are mandatory plan inputs. These checks repeat at phase 9.

The marker must be the actual compiled return-only function, with its source-gated optional CET ENDBR64 prefix. All 88 raw descriptor bytes, including unknown padding, remain recorded privately. Unit/Manager addresses are provenance, not general post-callback liveness proof; the adapter never dereferences the returned actor or Manager merely because the marker retains an address.

## Natural call attribution and hardware lanes

Phase 4 arms the complete original Manager entry. Its actual six argument registers must equal the live descriptor's Manager, Unit, Operation, NULL Plot, exact visibility flag and NULL result map. The actual stack return must equal the reviewed compiled caller return. It then arms Manager return and the source-closed script dispatch. Early busy/turn/disabled rejection can return without Lua: the adapter preserves that actual boolean and reports zero natural Lua calls, with no callback-absence claim.

When dispatch is reached, exact Manager continuation `XP2+0x2da22a0` and full caller suffix establish the original path. CCall entry additionally requires concrete direct parents `0x36b40e1` and `XP2+0x2ce79ff`, while the reviewed producer independently checks actual stack and context identities. The adapter records the pre-acquisition Lua lock, requiring no retained parent ownership and a nonnull initialized mutex pointer. At `0x3bf22fa`, after original Enter returned, it requires recursion one, a nonzero ASL ThreadHANDLE owner pointer and unchanged mutex identity. These facts combine actual normal acquisition flow, complete source-closed parent frames and raw lock fields. No GC control byte or owner/count sample substitutes for that provenance.

The producer then captures before lookup, actual naturally resolved function/groups when reached, protected result and completion before release, and original release/return facts. All threads must be stopped for every copied extent. Reads are bounded to the u64 address space, mapped readable extents and the producer's existing image budget. Main VM/global/collector bytes, raw named absence and actual uncached metamethod-returned functions retain their separate meanings. Lua owner pointers remain distinct from core u32 thread IDs and OS TIDs.

There is one persistent marker lane and at most two original target lanes. During the protected interval only the producer's next sites are armed; Manager return is temporarily unarmed when needed for the resolved/protected-return fork. It is restored after original wrapper return. A skipped/unwound Lua checkpoint therefore fails at the next marker or terminal check, never becoming an assumed callback outcome. After actual Manager return, phase 5 must agree with that observed AL result. Phase 9 requires completed Manager and original facade lifetime.

Every refusal writes a private failure row; producer failures preserve immutable partial images and terminal pending-call state. Successful terminal output records actual natural-call count, whether registry images were captured, and explicit absence of callback-effects/native-parity acceptance. A missing window is unavailable evidence. Save production and binary-identical rewriting remain decoding diagnostics, not action gates; complete affected gameplay/VM state, all RNG, order and logical identity still require comparison. The historical 89,198-byte raw Save→Load difference remains unclassified and unmasked.

Reproduce without original execution:

```sh
python3 tools/linux/full_manager_observer_runner_audit.py --output target/linux/full-manager-observer-runner-source.json
```
