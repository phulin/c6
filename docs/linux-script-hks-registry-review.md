# Independent Linux HKS image decoder review

Reviewed immutable producer commit `37c9c0fe66f3d6e0893f4a8f87a8cabb1372671e` (resolve the complete identity in [the evidence report](../evidence/linux/script-hks-registry-review.json)), including its predecessor `34822ca`. The review accepts the component as a bounded, offline immutable-image decoder. It does **not** accept a native registry capture, callback evaluation, full Manager execution or exact complete gameplay state/actions/RNG/turns. No original game code was executed. The historical **89,198-byte** raw Save→Load failure remains unclassified and unmasked.

The [independent review script](../evidence/linux/script-hks-registry-review.py) loads committed decoder source with `git show`; it does not import the mutable working-tree decoder or its fixture builder. Its own ELF program-header reader checks 46 host method/body fingerprints and two XP2 getter fingerprints against the pinned original files, including host ET_EXEC identity. This corroborates source identity, not behavior equivalence. Original instruction paths were also inspected independently for the conclusions below. Public evidence contains hashes, addresses and conclusions; original bodies are not reproduced.

## Findings

No blocking decoder defect was found within the documented static, shallow scope. A runtime-root assumption identified during producer self-review was corrected before this review's final snapshot: XP2 getters at `+0x2171660` and `+0x2103fb0` load the actual runtime GOT binding and then the global receiver pointer. Final decoding follows the captured GOT slots `base+0x3b12df0` and `base+0x3b13848`; it records whether their bindings equal the expected module definitions instead of assuming that equality. An independent fixture follows an interposed global. Host addresses remain absolute, with zero ET_EXEC load bias.

| Boundary | Independent source conclusion |
| --- | --- |
| TValue | Low four tag bits determine type; payload is at `+8` in a 16-byte value. Unknown tag bits and padding remain raw. |
| String/integer hash lookup | `0x3bbf9f0` compares intern pointer identity and tag4; `0x3bbfde0` compares tag3 and ordered binary64 numeric equality. Reverse links are at `nodes-8*(index+1)`. Existing string identity is required; scanning immutable live keys does not intern names. |
| Stable iteration | `0x3bc00f6` and `0x3bc01a2` visit nonnil array entries and then physical hash-node order. Collision-chain order is distinct. This conclusion applies to an unchanged table traversed from nil. |
| Native objlen | `0x3bb6dea` uses array-tail/binary search, hash exponential search and signed-overflow fallback. The fallback checks hash **key presence**, including nil-valued nodes. A targeted overflow fixture distinguishes this result from ordinary nonnil-prefix length. |
| C closure | `0x3bc0d52` allocates `0x30+16*n`, writes function/environment at `+0x10/+0x18`, i16 count at `+0x20`, and inline upvalues at `+0x30`. Negative counts are refused offline. |
| Lua closure/upvalues | `0x3bbf72c` loads Method u16 count at `+0x10` and allocates `0x38+8*n`. `0x3bc0e60` copies the current value into UpValue `+0x10` and changes the value pointer at `+0x20`; offline decoding distinguishes open and closed values and retains repeated pointer identity. |
| Nil hash string keys | `0x3b9e77a` skips marking both key and value when the hash value is nil. Such string pointers can be stale. Decoding preserves their raw nodes without dereferencing strings or assigning a query name to them. An unmapped near-u64-limit pointer is covered independently. |
| Builtin identity | `0x36b3960` installs one captured upvalue; `0x36b3540` iterates that group table, skips non-table groups, obtains local objlen, and fetches slots with raw integer lookup. Only the actual C function address identifies this builtin. Replaced C/Lua methods are retained without callback-vector interpretation; builtin missing group context is refused. |
| Null roots | `0x32463d0` unconditionally obtains singleton/state; `0x2fa2240` dereferences receiver `+0x10`. Null singleton/state and null runtime GOT binding are unsafe original call contexts, never evidence of empty registration state. |
| Lock boundary | `0x3bf22e0` enters/leaves Lua critical section `0x6ec9c70`; it is separate from the temporarily released/reacquired core lock. Core-idle checks do not prove VM/GC quiescence. |

The collision-chain checks apply to chains actually traversed. A successful native-style early match need not inspect a malformed trailing link; the decoder is not an independent validator of every pointer in an arbitrary heap graph. Likewise, shallow alias preservation does not prove coherent types, object lifetimes or complete transitive state. An immutable input permits safe repeat reads but does not establish that its producer copied one coherent live instant.

One capture-protocol documentation detail needs clarification: Lua closure decoding requires the referenced **Method prefix through `+0x12`**, containing its count at `+0x10`, even though Method bytecode/prototypes/debug data are unsupported. The decoder already fails if the needed count bytes are missing. This is a documentation followup, not a reason to admit partial images; the producer has been notified.

## Validation and limits

Run:

```sh
python3 evidence/linux/script-hks-registry-review.py --output /tmp/script-hks-registry-review.json
python3 -m unittest discover -s tools/linux -p test_script_hks_registry.py
python3 tools/linux/script_hks_registry_audit.py --output /tmp/script-hks-registry-abi.json
```

The independent review passes **14 tests**, including **511** array-hole patterns, signed objlen overflow with nil-valued hash keys, NaN collision behavior, reverse chains versus physical iteration, invalid links/cycles, unreachable and duplicate live intern identities, dead string pointers, shared upvalue aliases, missing/negative closure context, overlapping/truncated memory, actual interposed GOT bindings, null roots, and replaced builtin methods. Producer checks pass **16 tests** and the source fingerprint audit. These are source-derived synthetic checks; no original HKS registry image or differential/native oracle was available. They do not promote the component to native parity.

An actual safe-window main-VM capture remains unproven and absent. Static tables cannot reproduce callback-driven registration mutations, lazy `__index` execution, thread creation, protected error effects, allocations/GC, environment mutation, Method execution or gameplay RNG. The observer must not acquire new locks, invoke callbacks, force GC, or pin weak objects to manufacture a capture window. No runner or live scanner was changed. All player decisions remain external; unproven AI-owned data and unknown save bytes remain in exactness scope.
