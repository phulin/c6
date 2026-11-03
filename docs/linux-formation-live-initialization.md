# Linux Formation live initialization

`c6_core::formation_initialization` applies the source-proven unsaved Formation1
Read state alongside an owned decoded Formation. It preserves saved member,
path and cache order without cloning those vectors. The library still lacks
complete Civ VI gameplay state, accepted actions and complete turns; this local
initialization boundary does not establish those goals.

The static audit gates the supplied Linux XP2 SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and Linux host SHA-256
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
[The report](../evidence/linux/formation-live-initialization.json) records method
fingerprints, constructor-derived registration order and direct-call checks.
No original code is executed by this audit.

## Local constructor and Read boundary

Formation constructor `0x293b6f0` constructs one owning archive and seventeen
inline wrappers. Every wrapper passes that archive to its typed constructor;
the host base constructors `0x3c49c28`/`0x3c49c66` append the wrapper to the
archive's registration vector through `0x3c459e8`. The bool constructor argument
does not detach the wrapper: the host overload still registers unconditionally.
The wrapper constructors then store the same archive as their edit target.
Constructor order and inline address order agree. Archive header priority is 0.

| Index | Logical wrapper |
| --- | --- |
| 0–4 | ID, location, owner, moves remaining, domain |
| 5–12 | Eight saved Formation booleans, in wire order |
| 13 | Unsaved evaluating-operation boolean |
| 14 | Ordered last path |
| 15 | Last path cache location |
| 16 | Ordered unit member IDs |

Read `0x293b560` writes the saved payloads through wrapper `load` methods.
Those methods target the value directly; they do not mark the wrapper dirty.
Domain is the exception: the reader calls `edit()` before EnumFromStream, so
its index 4 is inserted into the owning archive's dirty-pointer set. After
reading members, the reader assigns false to evaluating-operation. Its
MoveAssignmentDo at `0x293ece0` inserts index 13 unconditionally, including the
fresh constructor's false-to-false case. The local Read dirty invocation trace
and unique membership are therefore `[4, 13]` for a newly constructed object.
The runtime exposes evaluating-operation separately from saved Formation data.

The owner postlude skips lookup only for signed owner -1. All other values go
to the actual Globals::GetPlayer provider; a null result remains null. The
initializer requires that provider explicitly and retains requested/resolved
logical identity. Saved roster presence cannot supply the lookup table.
Finally GetComponentID `0x293de70` constructs type 44 and SetID `0x3c470a6`
assigns its bits to the archive. ComponentID constructor `0x1fe4710` stores the
low sixteen bits of owner and type, followed by all 32 ID bits.

Enclosing Player::Formations Read `0x2b04da0` calls recycled collection Read
`0x2b050a0`. It performs AddAt and Formation Read in saved occupied-slot order,
then restores allocator/free-link metadata. Neither reader clears Formation
deltas or adds another sync registration after the per-object Read. This is
an **after-local/collection-Read** baseline, not a claim about any later global
load cleanup. Re-reading a preexisting object with earlier deltas is outside
`FormationRuntime::from_read`'s fresh-object contract.

## Owned edits and archive relationships

`LogicalArchive` owns the resolved ComponentID, complete ordered wrapper
registration and unique dirty membership. It retains invocation order
separately. Native dirty storage orders wrapper pointers; inline wrapper
address order is represented by constructor indices. Native allocator capacity
and addresses are not simulated.

`FormationRuntime::clear_path_cache` verifies the resolved object's identity,
marks path index 14, clears its vector, marks cache index 15 and writes invalid
coordinates (-1,-1). Empty paths and already invalid caches still cause both
edits. Repeated calls retain one entry per wrapper in dirty membership while
preserving each invocation in the trace. Evaluating-operation assignment
likewise records a dirty invocation even for an unchanged value. These edits
operate on the Formation already owned by World.

Lookup remains the existing World's native low16 lookup: original collection
Find `0x2b05830` excludes `0xffffffff`, indexes the low16 slot and returns its
current object without comparing generation. The sidecar belongs to that
**resolved** object. It must not bind the caller's possibly stale requested ID
as the resolved archive identity, or reject a valid Find because generations
differ. Directly passing a different object to an existing sidecar rejects
before mutation; that check does not replace World lookup.

Unit constructor registrations are also exported as `UNIT_FIELDS` (73 fields,
including activity 11, timer 32 and operations queue 72); its ComponentID type
is 1 (`0x20e82e0` through UnitID constructor `0x20bcfc0`). This establishes
logical attachment for those setters. `DirtyBaseline::Unavailable` explicitly
rejects queries for complete dirty membership when Unit Read/Reset or a later
load baseline has not been supplied. It still records subsequent known calls.
It does not assume the baseline is empty.

Archive attachment is distinct from sync-manager acceptance. SetID may remove
an earlier accepted registration, writes the new identity, and calls a present
sync interface's registration function for a valid ID. Acceptance is its actual
return value. The initializer does not fabricate that provider or acceptance,
and does not implement synchronous gameplay reporting subscribers. Those live
providers, global load cleanup, Unit initial dirty history, Manager membership,
registry/configuration identity and other action prerequisites remain explicit
integration dependencies. Static attachment facts alone do not authorize
accepted SkipTurn or replace its report execution with collected records.

## Checks

```sh
python3 tools/linux/formation_initialization_audit.py \
  --output evidence/linux/formation-live-initialization.json
cargo test -p c6-core formation_initialization --lib
```

Four source tests cover preservation of saved vectors, actual/null/no-owner
lookup, owner narrowing and archive identity bits, unconditional repeated
edits, unique dirty ordering, object mismatch without mutation and unavailable
Unit baseline rejection. They are source tests, not a native differential
oracle. Original Formation initialization fixtures and full affected-state
comparison remain an acceptance gap. Unknown save bytes remain unmasked;
the recorded raw Save→Load difference remains 89,198 bytes. Save production
is not a prerequisite for integrating this live-state boundary. All genuine
player decisions remain external.
