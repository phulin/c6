# Optional passive diplomacy-manager capture

`native/linux/diplomacy_manager_capture.[ch]` captures bounded raw state for the process-global Diplomacy.Manager. It is a standalone optional helper; the active full-Manager loader and CLI are unchanged. It does not call original getters, policy code, lock acquisition or gameplay methods and does not write original memory. No original run has used it at this checkpoint.

The caller must independently admit the exact module, configuration/inputs, retained Context/Game, genuine core lock, actual main thread and absence of competing mutators. It passes the already-proven module base and PlayerManager (Game+0x248), retains the original owners throughout, and supplies a field-only owner guard. The helper checks Linux TID and that guard on entry, before every read and at exit. A guard or custom reader is a caller trust boundary, not independent execution authentication. The default reader is `process_vm_readv` on this process; invalid addresses produce a partial failure rather than an unsafe C dereference. No native acquisition or release occurs inside the helper.

The outer finalized runtime must retain PID/process generation, stage, exact source/module/host/profile hashes, input and snapshot identities, context lifetime and closed raw output artifact. Equal snapshot bytes cannot transfer these process-global facts between runtimes. Stages are bounded token strings; the helper records actual module/context/Game/PlayerManager pointers and the main TID. A completed raw capture is not native gameplay admission or proof of a decision boundary.

## Exact recorded layout

| Owner | Source-backed fields |
| --- | --- |
| Open list, Core+`0x3c052f8` | libc++ prev+0, next+8, count+16; nodes retain links and unique Session pointer+16. |
| Closed list, Core+`0x3c05310` | Same libc++ layout; retain complete bounded sequence separately. |
| Active Session, Core+`0x3c05328` | Actual pointer, including null; nonnull referenced Session fields are captured. |
| Pending actions, Core+`0x3c05330` | EASTL vector begin/end/capacity/allocator; every 12-byte raw record preserves ID:u32, from:i32, to:i32. |
| Session prefix | Signed from/to at+0/+4; raw 24-byte **EASTL** pending header at+0x28. Its next is+0, prev+8 and allocator+16. There is no stored count in that header. Statement count/items/payload remain explicitly unknown. |
| Ever-alive, PlayerManager+`0x3a8` | Actual vector begin/end/capacity and ordered Player pointers, with each current ID+0xe0 and raw WasEverAlive byte+0x278. |
| Allocation slots, PlayerManager+8 | Actual 65-entry vector, retaining holes and slot64. InitializeLists visits only first64 allocation slots. |
| ID slots, PlayerManager+`0x20` | Distinct actual 65-entry vector. Never sort this to invent ever-alive allocation order. |

The [source contract](linux-diplomacy-manager-source.md) and [audit](../evidence/linux/diplomacy-manager-capture-audit.json) pin target bytes. PlayerManager.Initialize at `0x2b51770` explicitly resizes both pointer vectors to65. Session EASTL constructor, begin/increment and size bodies prove why its third word is allocator metadata rather than the count in the different global libc++ lists. Initial helper `58091c1` mislabeled those raw fields; corrective `3618c45` preserves their bytes and corrects labels. No original capture depended on the initial helper.

Limits are explicit: at most 8 MiB actual memory reads, 65,536 charged items and 4,096 nodes per global list, with caller-selected smaller aggregate caps. Checked pointer arithmetic, safe reads, count/tail/previous-link/cycle checks and vector stride/capacity checks stop at the first failure. The final record reports `complete:false` and its reason. Nonempty state is recorded; it does not veto or replace a gameplay predicate. Native empty-list claims require the real sentinel/count/link invariant, not a fabricated zero. The bounded failure is unavailable state, never an empty default.

## Decoder and tests

`tools/linux/diplomacy_manager_capture_decode.py` validates one bounded JSONL snapshot, complete framing and record order, actual target/profile/stage identities, raw-byte projections, list links and vector widths/counts. It retains partial raw prefixes without claiming emptiness. It preserves nonempty sessions and unknown Statement payloads; it never supplies a ProcessStatement or IsAI answer.

The decoder compares actual allocation/ID-pointer associations and first64 scan order against observed ever-alive handles. An unavailable or inconsistent initialized inventory remains explicit. The output's `native_execution_admitted:false` and `semantic_complete:false` do not become true merely because raw decoding succeeds. A later data adapter must validate the finalized capture/preimages and bind those facts once to its actual LoadedState dynamic witness. The [owned-state API](linux-diplomacy-manager-state.md) rejects runtime transplantation and requires exact Statement effects/external decisions for nonempty execution.

Synthetic tests exercise the C helper with byte-image readers and local synthetic `process_vm_readv`, plus decoder format/order/raw/count/pointer/budget/profile corruption, partial output and nonempty unknowns. They never load the supplied game module. Run:

```sh
python3 tools/linux/diplomacy_manager_capture_audit.py --output target/diplomacy-manager-capture-audit.json
python3 tools/check/test_diplomacy_manager_capture_decode.py
```

No actual current manager capture, original predicate comparison, native action/turn/RNG equivalence or UI requirement is introduced. The raw 89,198-byte discrepancy remains retained and unmasked.
