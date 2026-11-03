# Linux notification lifecycle implementation review

No blocking implementation finding remains in the declared ProcessAutoActivate-plus-Cleanup boundary at commit `1107fbe`. Independent verification reproduces candidate SHA-256 `72dc68a6d1fe890feaf5fd8cd45cf3096cd232665f2ef271ffcc921dafad0fc2`, identical source hashes and all **2,629** resulting native notification-owner bytes. All eight focused test groups pass. This is a diagnostic comparison under supplied scenario inputs: **actual native phase input parity remains false**, synchronous reporting remains undelivered, and complete gameplay state/actions/turns remain unsupported.

Review evidence is [notification-lifecycle-review.json](../evidence/linux/notification-lifecycle-review.json). The reviewed implementation and audit are [notification_lifecycle.rs](../crates/c6-core/src/notification_lifecycle.rs) and [notification_state_phase_audit.py](../tools/linux/notification_state_phase_audit.py). The target contracts are [notification-lifecycle.json](../evidence/linux/notification-lifecycle.json), pinned to the supplied Linux XP2 module and matching host. No native execution or Ghidra mutation occurred during this review.

The reviewed behavior preserves the original distinctions:

- Autoactivation uses signed severity against threshold 0/3 and the first matching override parameter. LockCount is signed and only positive values block. An eligible later locked record preempts an earlier retained unlocked record, including when already activated or dismissed; the implementation adds no dismissed filter.
- Base activation stores HasBeenActivated before synchronous delivery, supplies the actual bool argument, and rescans after delivery. The test subscriber changes a later record's eligibility and observes the prior store. Fixed external gates/configuration and owner-container identity are explicit hook preconditions; modeled record flags, definitions, locks and ordering can change before the next scan.
- Cleanup visits list order, transfers the logical object to its pool before owner-list erase, clears its pool association, preserves surviving order and leaves next ID unchanged. Logical object handles are distinct from duplicate notification IDs. Native allocator addresses are not required.
- Unknown required definition inputs reject; skipped definition reads can remain Unobserved. Unsupported bindings, ownership and initial delivery profiles reject before mutation. In-place budget exhaustion or a violated hook contract can leave a documented partial prefix and never count as native completion. The isolated adapter returns no candidate on error.
- The unsigned severity reduction for end-turn blocking is **not implemented here**. Neither are DefaultEndTurnBlocking +130, player blocking +7a8, SendAutoNotify or DismissExpired. Their omission is explicit; the signed selection rule must not be extended to blocking computation.

The review found one substantive gap before the final commit: the initial base-resource destruction profile accepted arbitrary transported parameter payloads, while native destruction can invoke owned payload behavior through VariantManager and virtual destructors. The final implementation rejects unpooled destruction outside canonical nonowned scalar32 branches before mutation. Activation-plus-cleanup preflights all potentially released nonpooled records; direct cleanup checks dismissed records. A regression test distinguishes rejected unpooled owned-payload destruction from pooled release, which retains the full payload without destruction. No remaining blanket payload-destruction assumption is used by this boundary.

The frozen fixture's default manifest still supplies hypothetical zero locks, missing definitions, unpooled base instances and permissive manager/player gates. Matching owner bytes does not certify those inputs, actual virtual bindings, missing live fields or pool state. The audit keeps actual_live_inputs_observed=false and actual_native_phase_input_parity=false even with a caller-supplied manifest; independent passive-observer provenance is needed before changing those claims. The corrected first-message/second-summary semantics from `2c30b37` are preserved.

## Exact NotificationActivated delivery assessment

Existing [passive reporting evidence](../evidence/linux/reporting-provider-passive-1.json) is more specific than a generic unresolved-callback warning. During its armed worker phase it records eight cache delegate calls belonging to other events, **zero notification activation delegate calls**, one actual NotificationActivated translator at host `0x3212d00`, and one Publish257 pre-publish getter whose pointer is null. The translator checkpoint precedes its indirect call, but the corresponding Publish257 observation corroborates the actual translation path. This closes the observed immediate cache-delegate/pre-publish callback branches for that invocation; it is not a universal registration inventory or a no-op assumption for other events/profiles.

The review additionally traces the exact activation sender's three payload keys through host EngineUtility::SendEvent (`0x32b4260`), with the full host SHA gate and method fingerprints retained in the review evidence:

| Payload key | Host path result |
| --- | --- |
| `a1e5a149`, sender metadata | Leaf `0x32b4ad8` converts uint, stores EventData +4, then advances the iterator |
| `eab69e5a`, notification identity | Unrecognized-key leaf `0x32b4bd0`, iterator advance |
| `78a2f946`, activation bool | Unrecognized-key leaf `0x32b4bd0`, iterator advance |

None of these paths calls DirtyComponents::Add. Event dirty-registration and parameter-usage lookups use find/read operations; their returned contents cannot select a dirty mutation for these three keys. The review evidence records each signed compare/branch path, entry/leaf addresses and relevant method hashes. This conclusion is specific to the exact three-key native activation report; it does not classify host reporting in general as pure output.

The observed translator validates the ID's native custom type, copies identity/bool and publishes KeepOrder host event 257. That logical queue insertion, its ordering and any later consumers remain obligations. The Rust diagnostic projection records the reporting request and does not execute this provider path or represent its complete queue state. Thus existing observations plus the new static key-path proof are sufficient to close the **immediate callback/dirty branch for the captured activation**, but they do not make the current Rust projection a complete delivered notification event or an accepted action. A concrete delivery model can now use that narrower fixture-specific contract without assuming general subscriber absence.

Reproduce the independent implementation check without original-code execution:

```sh
python3 tools/linux/notification_state_phase_audit.py \
  --iterations 20 --output /tmp/notification-independent-review.json
cargo test -p c6-core notification_lifecycle
```

The static key paths can be inspected with SHA-gated host disassembly over `0x32b4260..0x32b4c1c`; the evidence lists the exact comparisons and leaves for each input key. Original instructions remain private. The raw Save→Load **89,198-byte failure** remains unclassified and unmasked. Full manager Update, real live-input parity, complete RNG/subscriber effects and gameplay state/action/turn acceptance remain unresolved.
