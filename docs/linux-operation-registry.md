# Linux active Unit operation registry

The Rust implementation remains a bounded primitives, transport and rules/registry library. Complete live state reconstruction, gameplay actions and turns remain unimplemented. This document establishes original Linux registration and lookup contracts and a read-only native observer; static inventory alone does not establish the loaded active mapping.

The exact XP2 target is SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. Discovery first consulted cru121's pinned CC BY 4.0 reference at revision `5af389445118bed2bdebf7a8a42b219c8fba5175`; see [attribution and limitations](reference.md). Matching target DWARF and original instruction bytes independently establish the contracts below. Raw instructions, full class identities and the complete mapping remain in `../c6-ghidra/evidence/linux-operation-registry/`.

## Actual registration semantics

Manager Initialize (`0x2da13b0`) calls its Reset virtual and sizes its handler vector to the original UnitOperation definition count, filling with null pointers. Reset (`0x2da14b0`) returns immediately. DWARF establishes a 104-byte Manager, its 32-byte handler vector at `+24`, and its 48-byte hash map at `+56`. BaseHandler is 16 bytes, with runtime operation ID at `+8` and operation hash at `+12`.

Manager Register (`0x2da14c0`) rejects a null handler. Otherwise it obtains the handler hash through virtual `+0x10`, finds the original UnitOperation definition and obtains that definition's collection index. That **definition collection index** is the runtime operation ID. Registration position, vtable order, class name and hash sorting do not determine it.

On success, Register grows its vector if necessary, deletes the previous handler at that definition index through its original deleting destructor, stores the new handler, updates hash→index in the native map, and assigns the index to the handler's `+8`. A handler with no matching definition is deleted and registration returns false. Later successful registration for the same definition replaces the gameplay implementation. Manager Uninitialize deletes nonnull vector entries, clears the vector, then clears its hash map. The manager owns these handlers; an observer must borrow them without deleting them.

The exact XP2 Game InitializeComponents (`0x2d77d80`) has **70 actual Unit-operation Register calls and 66 distinct hashes**. This differs from the broader inventory's 88 concrete classes. Four later calls replace a previously registered same-hash handler in this initializer. The static audit retains the entire actual call order privately and resolves the last concrete implementation for each hash, independently of the loaded definition indices. It does not assume every possible definition exists or has a handler in every configuration.

## Const lookup and observation

Manager Get (`0x2da1260`) obtains the original game's component through GetComponentByIndex; it returns a borrowed const reference. No facade wrapper is acquired. Manager GetHandler(ID) const (`0x2da1650`) treats index-form IDs as vector indices without a bounds check; other HashID values dispatch to the hash lookup. The observer bounds every numeric call by the actual vector size first.

GetHandlerIDFromHash (`0x2da1720`) searches the original map and returns `-1` when absent. GetHandlerFromHash const (`0x2da16f0`) returns null on that `-1`, otherwise delegates to the const numeric lookup. GetHandlerHash (`0x2da1780`) is deliberately unused: although it checks numeric bounds, it dereferences an in-range slot without checking that the handler is nonnull.

`native/linux/operation_registry.c` uses original getters to obtain definition count/index/hash, handler vector count, each actual handler and the original reverse lookup. Its only direct object reads copy the actual handler vptr and three function pointers from that table: MakeParameters `+0x80`, node Write `+0xb0` and Read `+0xb8`. It records these identities without invoking a factory or serializer. Matching DWARF and byte fingerprints establish the receiver layouts and scalar SysV ABI. All engine objects remain borrowed; no engine allocations, mutations, wrapper releases, initialization, requests, message drains or updates occur in the helper.

The caller must enforce exact module and full configuration identities, successful lower-Core load, native synchronization and successful original cleanup. It must capture complete original Core Save streams immediately before and after this isolated query. Equal Core bytes establish serialized-Core nonmutation only; complete transient/live-state purity is a separate claim. This observer does not separately record the live Manager vptr or its delegated virtual getter bindings. Original creation/constructor evidence establishes the Manager class, while an explicit live binding observation remains an optional strengthening step; full transient-state purity is not proven. The static audit fingerprints the original getter methods and lookup chain and checks the actual constant-load getters and handler-store destinations; native evidence remains necessary for the loaded result.

Run the static audit:

```sh
python3 tools/linux/operation_registry_audit.py \
  --output target/linux/operation-registry-abi.json \
  --private-dir ../c6-ghidra/evidence/linux-operation-registry
```

The [bounded ABI report](../evidence/linux/operation-registry-abi.json) describes static inspection separately from the native observation. Native integration is owned by the existing loaded-fixture runner owner; this helper does not introduce a second runner or replace native queues. Its configuration key is `lower_core_operation_registry_observer`, containing the exact static audit result, plus the source identities of both helper files.

`tools/linux/operation_registry_report.py` requires the actual runtime report, before/after Save paths and a private output path. It checks exact source/config/module identities, complete cleanup, full actual Save equality, every definition slot including null handlers, original reverse lookups, the selected concrete vtable and factory/serializer bindings, and an actually absent hash/null boundary. It then checks every attributed queue from both original 1,262-Unit canonical streams against that observed active mapping, using exact original Save and serializer-trace identities. Unknown shapes or unresolved hashes fail. The supported corpus contains at most one node per queue; this checker explicitly rejects other shapes rather than pretending to parse them generically.

## Loaded benchmark observation

The [independently checked native observation](../evidence/linux/operation-registry-native-1.json), configuration `551e80945f1e8ffd28e70a74a10769a542314269d017ce348beb877ad20dee4f`, obtains **63 actual definition/handler slots and 63 active handlers**. Three statically registered hashes have no definition in this loaded profile. Every observed active concrete vtable and parameter factory matches the actual XP2 initializer's final selection, including the later same-hash overrides. No actual null slot occurs in this profile; the separately queried absent hash returns native ID `-1` and null handler. Absence from the profile is preserved as a coverage limit.

Both full 80,657,462-byte original Core Saves have SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`; complete native cleanup passes. All queues in **two captures × 1,262 Unit records** resolve through the observed active mapping. Each capture has 954 empty queues and 308 nonempty queues containing eight distinct wire hashes. The Unit payloads in these two captures are identical; this comparison does not supply independent branch diversity.

The existing selected-actor no-action control is also checked against the exact trace-owned Unit and queue ranges in both captures: native runtime ID 19 resolves to wire hash `580f2f68` (MoveTo), and directive, flags, push turn, saved activity and automation all match. The mapping independently resolves SkipTurn hash `2c681908` to runtime ID 36. This confirms the prior numeric conversion lead without treating either observation as the complete registry. No native queues or handlers were replaced to obtain this result.

Reproduce the native validation with the preserved private runtime and captures:

```sh
python3 tools/linux/operation_registry_report.py \
  --runtime-report ../c6-ghidra/evidence/linux-operation-registry/runtime-1.json \
  --before-save ../c6-ghidra/evidence/linux-action-boundary/operation-registry-1/save.bin \
  --after-save ../c6-ghidra/evidence/linux-action-boundary/operation-registry-1/repeat.bin \
  --private-output ../c6-ghidra/evidence/linux-operation-registry/native-1.json \
  --output target/linux/operation-registry-native-1.json
```

This registry does not make live parameters, queues or gameplay effects exact. Full initialized state, genuine action delivery, complete affected-state/RNG comparison and every-player decision yielding remain open. The raw Save→Load difference remains **89,198 bytes**, unclassified and unmasked.
