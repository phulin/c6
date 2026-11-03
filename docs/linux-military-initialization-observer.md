# Passive Linux Military observation boundary

The observer is prepared and statically tested; no genuine initialization snapshot
has been captured. It does not implement native initialization, Player/world import,
actions or turns. All fifteen AI wire codecs exist, but this observation boundary
must supply the attributed Military context before a complete Player wire result
can be accepted. The recorded 89,198-byte raw Save→Load difference remains unmasked.

The source contract is [78866b7](linux-ai-military-initialization.md). The target
is Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, with
host SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
SO addresses below are RVAs plus the actual load bias. The host is ET_EXEC;
its mapped `0x400000` is an absolute VA, not an additive symbol bias.

[The callback engine](../tools/linux/military_initialization_observer.py) and
[its audit](../tools/linux/military_initialization_observer_audit.py) never start,
resume or call an inferior, install breakpoints, execute an extra initializer/query,
or invoke an edit getter. `GetAllowedOperationTypes` at `0x21143c0` is forbidden:
it tails `FAutoVariable.edit` and can dirty the archive. The ordinary-load runner
owns its separately reviewed profile, genuine stream/lifetime markers and hardware
breakpoint integration. No UI, account, saved-Begin or alternate startup route is
introduced by this packet.

## Callback integration agreed with the original runner owner

The runner constructs `GdbAdapter(manifest_path, emit)` after the original module
is mapped, then calls `checkpoint(event, binding, frame_id)` while paused. `emit`
writes new private JSONL; never mix sessions in one output. The adapter checks
on-disk target/host hashes, actual host/module mappings, all-stop mode, full live
method fingerprints and a frozen runtime report/configuration/source gate.
It uses register and memory reads, never native expression evaluation. Preparation
produces no runtime gate, so its plan cannot attach until the runner supplies the
separately reviewed `runtime_gate`:

```text
report_path, report_sha256, configuration_sha256,
source_files: {absolute source path: SHA-256, ...}
```

The report's canonical configuration hash must match. The explicit profile key
`lower_core_military_initialization_observer.mode` must be
`ordinary_load_save_passive`; `core_component_mode`, the five movement/domain/
placement/placement-handler/unit-operation-manager metadata modes and
`full_save_request` must all be present and null. Every named source file must still match. `runner_source_sha256` below
is SHA-256 of canonical JSON of `source_files` (sorted keys, compact separators).
These checks establish identities; they do not authorize a new runtime profile.
The current packet has neither installed hooks nor changed a runner profile.

`Observer.observe(event, registers, thread, binding, frame_id)` is also available
for an independently gated caller. It returns false after a captured checkpoint;
true means refusal and the inferior must remain paused. Register names are integer
`pc,rsp,rax,rdi,rsi,rdx,rcx,r8,rbx,rbp,r14`. Entry IDs identify unique dynamic frames;
returns reuse their matching entry IDs. Branch/query/hash-array callbacks use the
owning InitializeLists/Write frame ID, even while paused in a called function.
Native returns require matching thread, frame, original stacked return PC and
`entry_rsp + 8`. Unwind, exception, missing return, destruction or unexpected
lifetime transition must call `abort(reason)`; process exit must call `close()`.
The runner must detect overlapping/cross-thread stream activity through its reserved
marker lane, rather than silently omitting a second root call.

Only InitializeLists nested directly under the actual Military.Read is admitted.
Standalone initialization is outside this capture boundary; its absence is not
claimed. No stream identity may be invented for a standalone call.

| Event | SO RVA | Observation |
| --- | --- | --- |
| `read_entry` | `0x2145620` | IStream `rdi`, Military `rsi`, all three prior vectors; prior owner pointer is recorded without dereferencing it |
| `initialize_entry` | `0x2145be0` | Military `rdi`, actual Player `rsi`; caller return must be `0x2145669`; complete prior vectors and actual owner |
| `personality_return` | `0x2145c07` | actual canonical native `al` result |
| `leader_return` | `0x2145c17` | actual signed `eax` leader index |
| `query_entry` | `0x203ac30` | original caller `0x2145c9b`; actual allowed-vector destination, handler query pointer/default, civilization and handicap arguments |
| `count_entry` | `0x2114220` | actual active definition collection receiver and ordered definitions |
| `count_return` | actual stacked return | original count result compared with witnessed collection span |
| `initialize_return` | `0x2145669` | matched normal completion, complete initialized vectors |
| `read_initialized` | `0x2145669` | same paused stop after `initialize_return`, now using parent Read frame ID |
| `read_return` | actual stacked return | complete post-Read vectors and owner |
| `write_entry` | `0x21453f0` | OStream `rdi`, Military `rsi`, complete writer-entry vectors |
| `write_guard` | `0x2145504` | actual allowed-size result; `rbx` Military and `r14` stream must match |
| `hash_array_entry` | `0x214d7b0` | ordered original callers `0x2145526/0x2145548/0x214556a`; actual stream/data/definition collection and all typed contents |
| `write_return` | actual stacked return | normal matched writer completion and all expected array callbacks |

The runner uses hardware breakpoints only, with no automatic software
FinishBreakpoint. Breakpoint creation/deletion/enabling occurs only in its outer
drain command after `stop()` unwinds. `HardwareSchedule` is a pure, offline tested
plan with at most three target lanes and one lane reserved for provenance markers.
Idle watches Read/Write entries. Read replaces them with Initialize entry and root
return; nested Initialize needs root return, Initialize return and one rotating
personality→leader→query/count lane. Count entry rotates to its actual return and
then disables that lane. False personality/null query need no count lane. Initialize
return drains its two callbacks before waiting for root Read return. Write needs
root return plus guard→hash-array lane; the third array disables that lane. Return
addresses come from the actual paused stack. The schedule is not a hook installer.

## Raw schema and stream attribution

Every row has schema `c6-linux-military-observation-v1`, sequence and event.
`observer_begin` records source/target/DWARF/plan identities and the frozen runtime
gate. Each root entry receives a fresh immutable binding:

```text
kind: read | write
occurrence_id, lifecycle_id, owner_lifetime_id
stream_pointer: actual native stream pointer
input_save_sha256, input_core_sha256
configuration_sha256, rules_inputs_sha256, dlc_sha256, mods_sha256
rules_database_sha256: actual DB byte snapshot digest, or null
live_database_snapshot_unavailable: true when no DB byte snapshot exists
runtime_profile_sha256, runner_source_sha256
output_id: provisional saved-output occurrence (write only)
```

All available digest fields are lowercase SHA-256. `rules_inputs_sha256` is the
actual frozen full-content input gate, not a fabricated in-memory gameplay DB
snapshot. A missing DB byte snapshot is explicitly unavailable; no SQL or dump
query is added. DLC/mod digests identify the exact input/activation scope. Empty DLC/mod lists still have an actual
canonical identity. These identities come from the genuine runner preimage and
stream/lifetime markers, not command-line booleans or observed serialized count.
The runner must bind the actual Military stream to its real MemoryI/OStream and
specific Save/Core input. A global fixture hash cannot silently stand in for a
different canonicalization stream. Nested initialization inherits only its matched
Read binding. Pointer reuse requires a new witnessed owner lifetime ID.

Snapshots contain `military_pointer, player_pointer, player_id_i32,
was_ever_alive, handicap_i32`, and `vectors.allowed/per_war/current`, each with:

```text
data_pointer, capacity_u32, size_u32, padding_u32, tail_padding_u32, values_i32
```

The vector data pointer is at `+0`, capacity is **u32** at `+8`, and length is u32
at `+16`; padding at `+12` must not be read as capacity bits. Values are signed
32-bit. Military vector offsets are `+0x158/+0x180/+0x1a8`; owner is `+0x98`.
The full bounded backing capacity must be readable; only initialized values are
exposed. Pointers/capacity/padding are native diagnostics, not portable state.
Read entry has separate `prior_vectors` and `prior_owner_pointer`; the saved owner
is not yet installed. Initialize entry is the attributed pre-initialization state.
No fresh-constructor, cleared-lifetime or default-zero state is assumed.

Definition observations preserve the ordered active pointer span and each actual
`database_row_i32, collection_index_u32, operation_type_hash_u32, default_value_i32`.
Their DWARF offsets are `+16/+20/+24/+28`, with a 40-byte definition object. The
active collection is its first std::vector pointer span at `+0/+8/+16`; indexes
are preserved as observed, without inferring contiguous database IDs. The actual
handler's query-time default is separately read as i32 at `+0x140`.
The count receiver must come from the actual original unknown-leader assignment
(return `0x2145dd2`, `rbx` allowed vector), or the witnessed nonnull allowed query
(return `0x203ac8b`, `rbx` handler, `rbp` allowed vector). The latter also validates
the actual standard count callable's vtable invoke slot `+0x30` against
`0x21141c0` and its captured definition pointer at `+8`. The full native dispatch,
invoke and wrapper bodies are fingerprinted. An unrelated/shared getter cannot
supply the count merely because an Initialize frame exists.
A branch that performs no count/query does not fabricate a definition/default
observation. A nonempty writer array shorter than the active definition count is
refused. All three writer helpers must witness the same ordered collection.

After original Save returns and disk close completes, the runner calls
`bind_output(occurrence_id, output_id, saved_output_sha256, output_core_sha256,
core_range)`. The half-open range is that Military record in the exact canonical
Core output. A provisional output ID alone is insufficient. The separate capture
review must verify those real artifacts, ranges and stream markers. A writer result
never supplies initialization context for another load or output occurrence.

These are raw observations, not the consumer's accepted InitializationSnapshot.
`capture_end.capture_complete` is always false: a separate audit must establish
all required per-owner/lifetime/input/read/output/write witnesses before promotion.
Unobserved definitions, standalone initialization, actual SQL readiness before a
call and complete gameplay semantics remain explicit gaps. The presence condition
is the witnessed initialized allowed-vector length being nonzero; count seven
in serialized data is never used to manufacture context. No native policy or RNG
call is added or treated as an external player decision.

## Reproduce preparation

```sh
python3 tools/linux/military_initialization_observer_audit.py \
  --output evidence/linux/military-initialization-observer-preparation.json
python3 tools/linux/military_initialization_observer.py \
  --module ../c6-assets-linux/libGameCore_XP2.so \
  --host ../c6-assets-linux/Civ6 --output /tmp/military-observer-plan.json
```

The audit checks 34 complete method fingerprints, the Player ID getter, the original
Military/vector/handler DWARF contract and added capacity/definition DWARF types.
Its 23 synthetic tests check typed bounds/signs, stream/frame/PC/RSP refusal,
branch/count handling, writer/output attribution and the three-lane hardware plan.
Synthetic memory is parser testing only, never an original native receiver or a
passing capture. No Ghidra/private project data or game binary is committed.
