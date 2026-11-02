# Narrow Linux action state observer

`native/linux/action_state.[ch]` observes a borrowed genuine original core
`Unit::Instance`, or only RNG when the unit argument is null. It performs no
request, update, drain, native policy, archive reset or per-variable hash call.
The caller must enforce the exact [Linux target](targets.md), configuration
identity, genuine loaded-object provenance and synchronized worker lifetime.
The observer itself does not establish that gate or initialize a session.

`c6_linux_action_state_capture(module, unit_or_null, stage)` emits one
`C6_LINUX_ACTION_STATE` JSON record to stderr. `stage` is escaped as JSON.
The separate `c6_linux_action_unit_queue_size(module, unit, &size)` supports
selection by actual original queue size. Neither function owns its inputs.
Return zero means observation succeeded; 210 rejects missing arguments, 211
missing original symbols, 212 absent original containers or nodes, and 213
RNG bytes changing during observation. A successful call is not action acceptance.

The record uses original activity, automation, operation-timer and movement
getters. Movement is an original four-byte FixedPoint result with nontrivial
C++ copy construction: hidden result RDI and unit RSI. Queue entries remain in
original `GetAt` order; every 40-byte node is retained alongside its scalar
fields. Raw pointer bytes are observations, not parameter semantics or complete
pointed-to state. A first fixture can select a genuinely empty queue; that does
not validate preexisting handlers or general queue mutation.

Archive observation uses original `GetAutoArchive`, `hasDeltas` and
`getDeltaIndexes`. The latter constructs its own 24-byte return vector in
aligned caller storage, with the archive in RSI. Original vector size and const
index methods expose every dirty variable index in original pointer-tree order;
the original nondeleting destructor releases the return vector. No sorting or
clearing occurs. Constructor registration indices must not be assumed stable
when original archive removal can compact its variable vector.

The observer records all 32 bytes of the actual Game Sync Random through the
**const** accessor, and all 32 bytes of `Map::Generator::ms_kRandom`, including
vptr, padding and the raw synchronous byte. It additionally reports seed, calls,
resets and lock fields, and compares both full objects before and after the
observation. No normalization is applied. These are two identified streams;
complete initialized-game gameplay RNG ownership remains unresolved.

Run `python3 tools/linux/action_state_audit.py --output evidence/linux/action-state-abi.json`
and `cc -std=c11 -Wall -Wextra -Werror -fsyntax-only native/linux/action_state.c`.
The audit hash-gates both original binaries, checks ABI and symbol contracts,
and verifies `getDeltaIndexes` direct memory stores target only the output
vector and stack. Its normal callee appends copied indices to output-owned
storage. Original allocation/free and stderr output affect host bookkeeping;
no claim of bitwise process purity is made. Private disassembly is under
`../c6-ghidra/evidence/linux-action-state/`.

Static checks do not replace native before/after Save, transient state and RNG
comparisons. This observer omits formation paths, notification/diplomacy state,
subscribers, caches, other archives and unclassified state. The existing raw
Save→Load 89,198-byte failure remains unresolved. The Rust implementation still
covers isolated primitives; this is not complete affected-state observation,
a mutating action adapter, or exact state/action/turn parity.
