# Independent review of the bounded Linux SkipTurn projection

Reviewed implementation checkpoint `cf8f762`, its production codecs, example, audit and checked report against the state contracts `f94e745`, `e0fd48d` and `41c17e9`. Corrected reporting evidence from `946babb` is considered separately below. This review changes only this document. No original code executes, no proprietary artifacts are copied or uploaded, and no implementation is changed.

## Result and finding

No blocking correctness defect was found within the declared Unit/Formation effect-projection contract. The supported path is a coherent operation-manager phase, not an accepted gameplay action. Complete gameplay state, action execution and turns remain unimplemented; `Engine::apply` continues returning Unsupported for gameplay actions.

One low-priority documentation finding remains: the module preamble in `crates/c6-core/src/skip_turn.rs` says queue replacement/activation remains outside the boundary and broadly requires synchronous effects hooks. That describes the independently callable leaves but is stale for `replace_operation`, which computes the bounded replacement/activation projection and returns undelivered effects through an internal collector. Its function-level contract and `docs/linux-skip-turn-kernel.md` correctly describe this distinction. Updating the preamble would prevent callers from confusing the two interfaces; this review makes no source edit.

## State and ordering checks

- MoveTo is recognized as an override, not treated as a direct BaseHandler binding: vtable `0x3affa30 + 16 + 0x68` resolves to `0x2da4b40`, which calls Placement::CompleteMovement before BaseHandler::Deactivate. The live getter reads Unit `+0x6d8`; its wrapper is `+0x6c8`, archive index 50, wire ordinal 34. The original record has false. A nonempty queue with true rejects before projection construction; the unmodeled identity/PlotCoord event path is not silently skipped.
- Clear on a nonempty queue calls manager deactivation before PopFront. OPERATION becomes literal AWAKE, independently of the old node's saved activity. BaseHandler reporting precedes DeleteNode; DeleteNode marks the queue before front deactivation/removal. New insertion precedes activation/Start; Start sets HOLD and delete=true while retaining action=false, followed by ordinary PopFront/deletion.
- Initial empty Clear returns without timer or path edits. Nonempty deactivation assigns timer zero unconditionally, including an already-zero value. Each path clear registers indices 14 and 15 even when already empty/invalid. The one-MoveTo path clears Formation three times: old deletion, Clear tail, new deletion. These repeated calls are instruction-derived requests, not captured native call counts.
- Unit registration indices 11/32/72 map to activity/timer/queue and wire ordinals 14/20/70, respectively. They are not interchangeable with wire positions. Ordered requests are `[11,32,72,32,72,11,11,72,32]`; their unique union with original dirtiness matches the native after membership. Native dirty-pointer tree iteration order and Formation dirty history remain unmodeled/unobserved.

All explicit preflight checks occur before cloning state or constructing effects: identity mismatch; unsupported request metadata; pending movement completion; automation; mismatched or nonsingleton Formation; unsupported handler/parameters or queue count; initial activity; busy or failed activation CanStart. Inputs are borrowed and no reporting hooks run on this projection path, so rejection cannot externally mutate them. Numeric handler identity, turn/local player, actual live predicate results and per-wrapper archive bindings are caller-supplied facts, not independently resolved by this API. Scripts, live registration changes and callbacks remain conditions on the bounded contract. There is no fixture unit-ID, actor-ID, coordinate, destination or notification-ID selection constant in the transition.

## Independent record replay and identities

Ran only:

```sh
python3 tools/linux/skip_turn_kernel_audit.py --iterations 0 \
  --output /tmp/c6-linux-skip-turn-kernel-review.json
```

The audit reads the immutable native captures and original ELF bytes, builds/runs the Rust example, and executes no original game code. It passed, including all 16 synthetic checks. Its candidate SHA-256 equals the frozen report's `2113136fdca4b8aca46657125b2885653e503ee7826dd0567598d8388a3f6ca9`; the only report difference is the iteration/timing object. All six current source SHA-256 values and the state-report SHA-256 also match the checked [kernel report](../evidence/linux/skip-turn-kernel.json).

The original module gate is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, and the historical configuration is `d68604a55344b85212154f5abf20b2a109e2d65b34a582686226e1f3f520187a`. The adapter verifies complete before/after stream identities, attributed record slices, actual command identity, selected original method fingerprints and MoveTo vtable binding. It uses production message/Unit/Formation codecs and observed turn/local player rather than fixture identity constants. Positive activation predicates are inferred from the original executed branches; Formation archive bindings remain assumed for static request comparison.

Exact complete output equality was re-established for:

| Record | Output bytes | SHA-256 |
|---|---:|---|
| Unit, all 73 wire fields | 4,774 | `70d8db374d99b5029cbfa77c1ac4ec8bb36eb6f03d56ce55c01846a4fec8786d` |
| Formation | 56 | `a744ba02f244cfe8b8255966e85229a0880de139fef7dce3d2e7c2ed6a446b78` |

The underlying original result is one actual action profile with a fresh isolated repeat, not a native boundary matrix. The 16 modified-input checks are Rust/static-contract checks without corresponding native boundary fixtures. They do not expand accepted native coverage.

## Reporting and remaining acceptance gaps

The projection's nine identity reports match the historical selected arguments/order. The historical wrappers have the documented int/void ABI defect; their selected trace is diagnostic evidence. Separately, [the corrected observer comparison](../evidence/linux/core-component-event-historical-comparison-1.json) from `946babb` establishes the same actual command, complete before/after/repeat streams and first nine selected entries under configuration `b6ca58d53073abc2afa9176a4ceba350fafe3f91ba12dbf84fd08e25a3be9a5b`. That corrected execution and repeat capture eleven balanced entries, all returning original int zero. Entry 10 is notification activation `0x025f6cfd`; entry 11 is queue-drain `0x9ea4c222`. Both are outside the operation projection. The bound blocker overload emits no entry in this profile. This separate comparison does not convert the kernel report's historical diagnostic flag into corrected-observer parity or exercise nonzero returns.

Original Cache dispatcher, provider/translation and pre-publish callbacks can run synchronously. Returning reports cannot fulfill those callbacks or prove they leave inputs unchanged. Actual provider flags, callback targets and transitive effects require their own evidence; investigations in progress do not silently become accepted contracts here. The standalone Start leaf does correctly reread activity after its synchronous send, and its synthetic subscriber check is explicitly synthetic.

Full legality, actual archive registration/tree state, omitted Formation live state, diplomacy, notification/player aftermath, complete event delivery, complete gameplay RNG and all external player-decision boundaries remain required. No full action, turn, legality or full affected-state parity follows from these matching wire records. Unknown bytes remain unclassified and unmasked: the action fixture retains its 46,038,925-byte raw positional difference, and the separate historical 89,198-byte raw Save→Load failure remains unresolved.
