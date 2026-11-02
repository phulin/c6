# Linux SkipTurn first conditional state diagnostic

The first separately admitted Core command **did not execute SkipTurn**. It queued one genuine command, but original Receive and UnitOperation execution were never observed. Native result 230 and incomplete controller teardown remain failures. The [checked artifact report](../evidence/linux/skip-turn-state-execute-failure-1.json) validates that diagnostic; `artifact_checks_passed` does not mean a native action passed. Rust still supports bounded primitives, wire transport and rules/registry components; complete gameplay state, actions and turns remain unimplemented.

This is the [conditional Core component](linux-core-component.md), configuration `5f87646b4b0ff97f755918703a70bc1062e04d33e9d57565c157a1edb1c6d18c`, against Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` and its pinned host. It bypasses original host admission by supplying that component's stated preconditions. Full-host admission, full action acceptance and gameplay parity remain false/zero. The separate host loading-deferral failure remains unresolved.

## Actual failure and observed state

Original MakeRequest accepted operation 36, flags 0, append=false and fromUser=false. Original serialization produced the 28-byte UnitOperation with actual owner 0 and unit 15794187. Queue captured identical bytes and sender. Trace phases 5/6 record the threaded update/synchronization call boundary, followed by phase 7 observations; a phase marker proves neither worker execution nor completed gameplay work.

The closed trace ends with queued=1, received=0, unit_executed=0 and reports=0. Pending Core messages changed from 0 to 1. The first failed postcondition is therefore the undrained message after those original entry points returned. The original cause is not established by this artifact. Later Uninitialize reports initialized=0, context_bound=0, **executing=1 and cleanup=0**. This is a failed partial native component state, with message/controller state effects outside the captured serialized Core; it is not a nonmutating Unsupported result or a resumable external-policy decision.

Before, after and repeated-after Core Saves are each 80,657,462 bytes with SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`. Raw common-byte differences, length differences, byte shifts and masked bytes are all zero. The outer final repeat Save is absent because the helper failed. Four selected-unit observations match exactly, including queue raw40 bytes, native dirty-index order and both identified RNG raw32 objects. Turn 592, phase 0, local player 0 and observed update/advance/AI/turn-processing flags remain unchanged. The two diplomacy observations have no pending actions or nonempty open-session map; because UnitOperation never ran, this does not validate its diplomacy drain or notification updates.

No queue replacement, cancellation, activity change, timer assignment, formation clear, dirty-index insertion or reporting event was observed. Complete live state and complete gameplay RNG remain unobserved; unchanged streams do not establish their equality. No state has been classified as presentation-only or planning-only.

## Typed initial contract for the next execution

The audit matches the complete before Save hash to the original serializer ownership trace used by the [Unit/Formation codecs](linux-unit-codec.md). It extracts those exact original ranges, decodes and roundtrips all 1,262 Units and 1,250 Formations, then selects by decoded `(owner,id)`. It checks actual encoded command identity, native actor selection/legality and live scalar observations. It never searches for ID bytes or infers record boundaries from object offsets. All 73 selected Unit field extents and their hashes are retained in the report. Identical complete after streams prove these same ranges remain valid in this diagnostic.

| Actual typed record | Original Core half-open range | Initial values |
|---|---|---|
| Unit `(0,15794187)` | `[5052651,5057469)` | OPERATION `0x490cc8e1`, automation -1, timer 0, raw moves 256, formation 16842764 |
| Its operation queue | `[5057078,5057130)` | One MoveTo, hash `0x580f2f68`; native runtime ID 19 is separately established by the registry observation |
| Its Formation `(0,16842764)` | `[4697095,4697251)` | Five path nodes, cache location `(85,58)`, ordered unit IDs `[15794187]` |

The MoveTo node has directive -1, flags 4104, push turn 591, saved activity `0x6b25012e`, saved automation -1, and typed MoveTo parameters location `(85,58)`, modifiers 4104. Parameters come from the actual serialized handler body, not the live node's pointer bytes. Their complete live ownership/pointee equivalence remains unproven.

The actual Formation path preserves this original order; each node uses the version-1 20-byte wire representation. `data1_raw` retains the original fixed-point bits.

| Index | Location | data1_raw | data2 | Flags |
|---:|---|---:|---:|---:|
| 0 | `(85,58)` | 256 | 4 | 0 |
| 1 | `(84,58)` | 0 | 3 | 0 |
| 2 | `(83,58)` | 256 | 3 | 0 |
| 3 | `(82,58)` | 0 | 2 | 0 |
| 4 | `(81,58)` | 256 | 1 | 0 |

These numbers describe this identified fixture only. The next fresh execution must discover its actual actor and match the relevant typed state rather than hard-code this ID. Its preexisting MoveTo and populated formation path make an empty-queue-only transition insufficient for this fixture.

The instruction-derived implementation requirements remain those of [SkipTurn](linux-skip-turn.md) and [archive tracking](linux-unit-archive.md):

1. Append=false clears the existing queue through original MoveTo deactivation/removal and clears the actual formation path cache. Exact old-handler effects must be modeled before accepting this initial state.
2. Tracked queue edits, timer assignment (including zero to zero), activity transitions and original ordered reporting effects must survive even where final values coincide. Preserve existing dirty history; do not replace it with a final-value diff. The current dirty vector contains neither activity index 11, timer 32 nor queue 72, so these future tracked insertions are observable if the intact registration mapping is confirmed.
3. For an otherwise normal formation clear, its five path nodes become empty and cache coordinates become `(-9999,-9999)`, with Formation tracked fields 14/15 marked. Removing five 20-byte serialized nodes would shorten this Formation record by 100 bytes. This is a **static prediction**, not an observed changed stream or permission to subtract 100 from every later offset.
4. SkipTurn's immediate Start sets HOLD and deletes its new node. Preserve the earlier old-handler, activity, reporting, timer and formation effects in their original order. The original UnitOperation then processes pending diplomacy and owner end-turn blocking/notifications; all gameplay effects remain in scope and every player decision must yield externally.

No accepted Rust transition can be derived from this failed fixture. The next useful native result is a proven worker Receive/UnitOperation execution with balanced completion and valid teardown; then compare the full affected typed state and ordered events. For a changed Save, establish container framing or after serializer-owned boundaries before decoding shifted records. Preserve every raw difference and every unresolved byte. Do not patch opaque saves to manufacture poststate.

## Reproduction and limits

Once the runtime owner has closed the process and its files:

```sh
python3 tools/linux/skip_turn_state_audit.py \
  --runtime-report /path/to/closed-core-component-execute-1.json \
  --output target/linux/skip-turn-state-execute-failure-1.json
```

This checker executes the Rust wire codecs only. It hash-checks the runtime configuration, reviewed native source contract, archived preload, actual original input, command, queue packet, snapshots and trace. Its deliberately bounded profile rejects changed streams, received/executed messages, mixed host profiles and any other diagnostic, so it cannot silently reuse old offsets for a later success. A future successful state audit requires a separate extension backed by its actual evidence.

Validation reproduced all typed roundtrips and the unchanged streams/observations. Altered configuration identity, altered module identity with a recomputed configuration hash, and altered command-artifact identity all reject. Python compilation and whitespace checks pass. No gameplay implementation changed, so this checkpoint does not add gameplay parity tests.

Unknown bytes remain unclassified and unmasked. The historical raw Save→Load failure remains **89,198 bytes**. No new original-code run, Ghidra mutation, native policy choice, transition port or action parity claim is made here. Original-code contract attribution and reference limitations remain in [reference.md](reference.md).
