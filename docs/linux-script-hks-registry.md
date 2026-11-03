# Passive Linux HKS registration reconstruction

An [offline image decoder](../tools/linux/script_hks_registry.py) now reconstructs the pinned Linux HKS table layout, raw named event lookup, stable iteration order and shallow callback closure/upvalue state. It never reads a live process, calls Lua/native code, acquires locks or executes callbacks. The [ABI audit](../evidence/linux/script-hks-registry-abi.json) fingerprints 46 host methods/bodies and both XP2 globals/getters. **No actual main-VM registry image exists yet.** FullManager, current-handler interruption effects, callback scripts/effects, complete gameplay state, actions, all RNG and turns remain unsupported. The historical **89,198-byte** raw Save→Load failure remains unclassified and unmasked.

This extends the [concrete script backend audit](linux-script-can-start-boundary.md); it does not replace its captured profile or claim an initialized Lua VM from ScriptSystem presence.

## Exact roots and null branches

| Source-closed location | Meaning |
| --- | --- |
| XP2 base `+0x3b12df0` | Actual runtime GOT binding to ScriptSystem global |
| XP2 base `+0x3b13848` | Actual runtime GOT binding to EngineUtility global |
| XP2 base `+0x3c051c0` | Defined ScriptSystem global; expected binding target |
| XP2 base `+0x3c051b8` | Defined EngineUtility global; expected binding target |
| Host absolute `0x6ec9e68` | Main LuaSystem singleton pointer |
| Singleton `+0x10` | Main lua_State pointer |
| lua_State `+0x70` | Main global HksObject |

The host is ET_EXEC: symbols are already absolute addresses, and load bias is zero. XP2 is ET_DYN; add its actual module base. The GOT relocations for both XP2 globals are original GLOB_DAT bindings. The decoder follows the actual copied GOT pointer before reading each global; defined symbol addresses do not rule out ELF interposition. It records whether that actual binding matches the expected XP2 definition. A null GOT binding is an unsafe native getter context, not a null ScriptSystem observation. The decoder records actual receiver/vptr values and compares them to the established ScriptSystem (`0x53a63a8`), EngineUtility (`0x53a6c90`) and AppLuaScriptSystem (`0x53aaa00`) address points. It does not call the host's lazy GetScriptSystem factory.

Manager skips scripting only when its initially resolved ScriptSystem is null. Concrete ScriptSystem::CallTestAll unconditionally gets the main Lua singleton and calls GetState, whose body dereferences singleton `+0x10`. A null singleton therefore has an unsafe native dereference. A nonnull singleton with null state enters CCallWithErrorHandling and original cclosure/cpcall allocation without a null-state guard. Neither condition means an empty callback registry, a false Lua result or legal operation. The decoder records null roots without dereferencing them and marks event inspection unavailable. Nonnull state alone is not initialization proof.

## Concrete memory representation

HksObject is 16 bytes: raw u32 type at `+0`, unknown/padding bytes at `+4`, and an eight-byte payload at `+8`. Its semantic type is the low four type bits; the decoder retains all raw bytes and does not mask unknown storage. Relevant tags are number 3, interned string 4, HashTable 5, Lua closure 9 and C closure 10. Numeric payloads are IEEE binary64; table and closure payloads are pointers. It does not reinterpret other tags as ordinary callbacks.

A HashTable header occupies `0x40` bytes. Metatable is at `+0x10`, u32 hash mask at `+0x1c`, node pointer at `+0x20`, array pointer at `+0x28` and u32 array extent at `+0x30`. Mask `0xffffffff` denotes no hash nodes. Otherwise node count is mask+1, a power of two. Each node is 32 bytes: key HksObject followed by value HksObject. Eight-byte collision links precede the node block in reverse order: node i's link is at `nodes-8*(i+1)`. The complete hash allocation therefore requires `40*(mask+1)` bytes beginning at `nodes-8*(mask+1)`. Array storage has `16*array_extent` bytes. Unknown header fields, links, nil slots and node bytes are retained.

InternString stores length/flags at `+8` (low 62 bits are length), stored hash at `+0x10`, and bytes at `+0x14`, followed by a zero terminator. Existing intern identity is essential: native string lookup uses pointer equality plus low-four-bit type equality. Its natural bucket is `(pointer ^ (pointer>>5) ^ (pointer>>8)) & mask`. The decoder scans copied live string-key nodes to identify the existing intern pointer by its bytes, then follows the original collision chain. It never interns the query name. Duplicate intern identities, unreachable matches, cycles, invalid extents and uncaptured memory are errors.

Nil-valued string-key nodes need special handling. Original GC `markTable_common` (`0x3b9e6d0`) skips both key and value when the hash value is nil. Their key pointer can therefore outlive the string. The decoder preserves those nodes raw and never dereferences their string pointers. When no nonnil matching key is found but such nodes exist, it reports `absent_or_nil_raw_key` and the unclassified node indices. Only a table with no matching live key and no such ambiguity reports `absent_raw_key`. Both are raw nil lookup outcomes; neither authorizes ordinary metamethod lookup or establishes an absent callback system. Full interner/GC lifetime reconstruction is a remaining dependency.

Stable `lua_next` traversal from nil visits nonnil array slots in ascending index order and then nonnil hash values in ascending physical node order. It does not sort hash keys or follow collision chains for iteration. The decoder implements that order for an immutable HashTable. It also implements bounded integer raw lookup and the original `contiguousArraySize` binary/exponential search, including hash extension and the signed-overflow fallback. A table with holes does not have a unique intuitive “list length”: the implementation's boundary search is preserved, rather than replacing it with entry count or first hole. These stable-image operations do not simulate callbacks mutating a table during iteration.

C closure allocation is `0x30+16*n`: native function pointer `+0x10`, environment table pointer `+0x18`, signed i16 upvalue count `+0x20`, and inline HksObject upvalues beginning at `+0x30`. Lua closure allocation is `0x38+8*n`: Method pointer `+0x10`, environment `+0x18`, and UpValue pointers beginning at `+0x38`. Its Method stores u16 upvalue count at `+0x10`. Each UpValue's `+0x20` points to its current HksObject; closed values reside inline at `+0x10`, while open values can point into a thread stack. The decoder preserves that distinction and copies the current value; it never closes or pins an upvalue. Method bytecode/constants/prototypes/debug data, transitive environments and arbitrary VM object graphs remain unimplemented.

## Named event interpretation

The decoder reads raw `GameEvents`, `GameEventFactory`, `CanStartOperation` and `TestAll` keys, recording raw tables, metatables and any raw `__index` closure identity/upvalues. It does not use ordinary global/field lookup, Count, factory helpers or Lua stack APIs. Missing raw event state remains missing in the snapshot, even when the captured metatable points to the native lazy indexer. The reader never materializes default events or interprets a nil raw value as proof that an ordinary lookup would have no effects.

Only a C closure whose actual function pointer equals the pinned built-in `lTestAll` (`0x36b3540`) is interpreted as that built-in dispatcher. Its first upvalue must be the shared group table. The decoder copies that table in original iteration order, each table-valued callback group, native objlen and numeric slots 1..N, including non-callable values. C/Lua callback closure identities and shallow upvalues are copied when present. A replaced function, Lua closure, missing method or non-table raw value remains explicitly different; it is not coerced into a built-in callback vector. Function identity and copied registration storage do not establish callback script behavior, protected error effects, allocation/GC effects or RNG semantics. All player decisions remain external; native AI policy is not executed to close gaps.

## Proposed snapshot protocol for the sole runner

This protocol is reviewable preparation, **not authorization to execute a new capture or alter the runner**. The existing Manager native-1 capture lacks these bytes. Its guard establishes controller/context idle, no queued messages and no pending game/update/AI processing. It does not establish main Lua/GC quiescence or registration lifetime.

There are two independent native locks. The XP2 wrapper temporarily releases an already-held core lock before scripting and reacquires it afterward. Original CCallWithErrorHandling (`0x3bf22e0`) enters/leaves the separate global Lua critical section at host `0x6ec9c70`. An observer must not acquire extra locks, invoke the dispatcher to obtain that lock, force GC or pin objects merely to inspect. The next runner preparation must first source-close a preexisting synchronous main-VM safe window where no callback, VM mutator, GC or competing owner can advance during the copy. Core-idle checks and two equal byte passes alone do not prove that window. No live snapshot helper is shipped because this original safe execution boundary is still missing.

At that admitted window, copy bounded mapped memory into private immutable artifacts and decode offline. Start with both runtime GOT slots and their actual bound global addresses, the main singleton global, actual ScriptSystem/utility vptrs, the singleton prefix through `+0x18`, and main state prefix through `+0x80` when present. Preserve nulls and unknown/overridden vptrs without calling any getter. Also record observer thread identity, active controller and core lock owner/count; preserve original Lua lock/lifecycle provenance without changing lock state.

For each table required by raw lookup/provenance, copy its entire `0x40` header, physical array extent, reverse links plus all hash nodes, and each nonnil string-key object's header/bytes/terminator. Copy GameEvents/GameEventFactory, their metatables/indexer closures, the named event/method and all built-in shared groups/local arrays. Copy each closure through its bounded upvalues and each referenced UpValue through `+0x28` plus its current HksObject. Preserve raw non-table and non-function values, tombstones and unknown fields. Future callback-state work needs the Method code/prototype/debug data, source provenance and transitive captured environments; this decoder does not claim to reconstruct them. Repeated references must retain identity; do not turn borrowed pointers into newly registered objects or root weak group tables.

Use before/control/after/repeat raw Core Saves, known RNG controls and a complete repeat snapshot as separate controls. Snapshot pointer equality is meaningful within this one process/lifetime; it is not a cross-run object identity. Keep original bodies, heap bytes, callback bytecode and script artifacts private. Public evidence should contain identities, sizes, typed conclusions and unresolved fields. Preserve partial/failing images privately; never report truncated output as accepted or silently replace missing bytes with zero.

The decoder accepts this minimal manifest beside hash-identified memory segments:

```json
{
  "schema": "c6-linux-script-hks-image-v1",
  "host_sha256": "106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4",
  "gamecore_sha256": "a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa",
  "configuration_sha256": "<actual configuration SHA-256>",
  "host_load_bias": 0,
  "gamecore_base": "<actual XP2 base>",
  "segments": [
    {"address": "<copied virtual address>", "file": "<private segment file>", "size": 0, "sha256": "<segment SHA-256>"}
  ]
}
```

The example is a schema illustration, not a runnable fixture or invented capture. Segment ranges must not overlap and each requested object extent must be covered by one complete segment. Coalesce adjacent copied ranges or emit complete bounded object allocations. The reader fails on missing memory, inconsistent masks/pointers, excessive extents or invalid links. A manifest's claimed hashes identify inputs; they do not prove the producer respected the required window. Output always marks capture-boundary acceptance false and gameplay unsupported.

Reproduce the source/boundary checks:

```sh
python3 -m unittest discover -s tools/linux -p test_script_hks_registry.py
python3 tools/linux/script_hks_registry_audit.py --output target/linux/script-hks-registry-abi.json
```

After a separately accepted private image exists, offline decoding is:

```sh
python3 tools/linux/script_hks_registry.py --image /private/capture/image.json --output /private/capture/decoded.json
```

Sixteen source-derived immutable-memory tests cover exact raw-byte retention, embedded-zero strings, array/hash ordering, collision links, holes/hash length, stale nil-key preservation, malformed/missing memory, open/closed upvalues, absent raw events with a lazy indexer, built-in versus overwritten methods null roots and runtime GOT interposition. No original HKS image/native comparison has run. Existing Manager adapters, frozen candidates and runtime runner are unchanged by this work.
