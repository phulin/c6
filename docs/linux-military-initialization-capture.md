# Closed Linux Military initialization capture

The closed `native-2` observation contains genuine framing context for 43 incoming
Military Reads and 86 outgoing Military Writes. The incoming Core is `a66736f0…`;
the two actual outgoing Core files are both `ff31f4c5…`. These are separate role
and stream occurrences. There is no observed `ff31` Read initialization here.
This closes a bounded observation gap; complete gameplay state/RNG, actions and
turns remain unsupported. The historical 89,198-byte raw Save→Load failure remains
unmasked.

The [offline classifier](../tools/linux/military_initialization_capture_audit.py)
checks the closed artifacts and raw witnesses without invoking original code or
a candidate. Its [derived report](../evidence/linux/military-initialization-capture.json)
records per-occurrence owner/range/context hashes and all remaining limits. Raw
observations remain private and unchanged, including `capture_end.capture_complete
= false`; no typed Rust snapshot admission or simulator comparison is performed
by this classifier.

## Artifact and profile identity

The original runner's closed capture is
`../c6-ghidra/evidence/linux-military-initialization/native-2`. Its runtime report
has SHA-256 `f45554791cb9aeee891fbe36ac8ec0d6ef859fbd960f4f6819c3fb60281246bd`;
its 3,057-row JSONL has SHA-256
`3bb06efe0a31da5e215a2e553e9f3b7bb41ef6da6ffc4cd93a5f4972330addb0`.
The frozen ordinary Load/Save profile is
`89aced529f2fe26bfb62eac23240e21b36cf031d055b506a1c3f486e489e5344`.
The supplied Linux XP2 module and matching host remain pinned to
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.

The classifier rehashes every declared closed artifact, the exact input files,
both complete actual output files, the host and module. It checks the runtime
configuration against its prepared report and raw observer plan, and verifies all
34 captured source preimages against their archived and unpacked bytes. The source
archive SHA-256 is
`4c4cc51fc4d428c59db17c40010bd3ab103ece076a8908cc4e92da58f667d177`.
It does not silently replace captured sources with the current checkout.

The original process and debugger exited zero; the terminal supervisor records no
remaining session processes. Startup, Shutdown, framework cleanup and client cleanup
returned. Controller Initialize/Uninitialize were not invoked. All 917 paused
stops have the source-reviewed worker/controller guard: worker pointer null and
controller initialized byte zero. The final recorded original-code fingerprint
guard covers 34 methods. These are attributed source-guard witnesses, not a new
execution or an independent offline dump of live process memory.

## Observed contexts

| Stream occurrence | Actual role | Core bytes | Military records |
| --- | --- | ---: | ---: |
| 1 | Incoming original Load | 76,556,938 | 43 |
| 2 | First actual original Save output | 80,657,462 | 43 |
| 3 | Second actual original Save output | 80,657,462 | 43 |

The incoming Core SHA-256 is
`a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa`.
Each actual outgoing Core file has SHA-256
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
The separate 7,348,717-byte whole Save supplied to the saved-configuration loader
has SHA-256 `a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8`.
Its extraction relationship to the actual incoming Core is explicitly unproven;
the two input identities are never substituted for one another.

The parser checks all 129 original Military stream ranges against their actual
entry/return offsets. Each stream has its own ready/return/release/destruction
witnesses. Each output additionally has file-close and per-Military output-hash
bindings. The physical stream pointer is reused between streams; occurrence IDs
and destruction markers keep those lifetimes distinct. The raw file-close
`marker_checkpoint` descriptor appears before its output-binding rows and trailing
human-readable `stream_marker`, exactly as the frozen runner emits them.

Within each Read, the classifier checks entry prior vectors, saved-owner
installation, nested InitializeLists entry, actual personality/leader/query/count
observations, normal Initialize return, the immediately paused initialized snapshot,
and normal Read return. Native frames retain their entry return PC, thread and
stack identities. Nested stack extents are checked against the pinned prologues.
`read_initialized` shares its paused checkpoint with Initialize return and uses the
parent Read frame ID; it does not have a separate checkpoint row.

All 43 observed personality results are true. Their actual leader indices are
known, their actual allowed-operation query pointers are nonnull, and their
original count callbacks observe seven active definitions. All 43 allowed vectors
have initialized size seven, and the actual query-time handler default is zero.
Those facts come from the native calls and typed vectors, not saved array counts.
The complete ordered definition identity is identical across the 86 count
entry/return snapshots and 258 actual writer-helper snapshots. Its portable order
hash is `e895435ea6ff98783727bd6334736a39d94ca0413ac00e68f9675fe90c0f49c2`.
The raw definition field named `default_value_i32` preserves DWARF `m_Value` at
`+28`; the actual handler's resize default is the separate observed zero at
handler `+0x140`.

All three vectors are retained at their actual prior, initialized and post-Read
stages with exact signed values, u32 lengths/capacities, pointers and padding.
The source-required current-vector zero initialization is checked. Twenty current
vectors then change while original Read consumes saved state. The observed allowed
and per-war vector values remain equal across those two stages for this fixture.
A post-Read snapshot therefore cannot replace the witnessed initialization stage.
The separate per-war query/preparation/rows were not hooked; its complete normal
Initialize-return snapshot is retained without claiming to replay its SQL.

For each Write, entry, actual presence guard, three ordered helper calls and normal
return are checked together. Each helper's actual stream, data pointer, values and
ordered definition collection match its own writer occurrence. All 86 guards have
nonempty allowed vectors. A writer context belongs to that specific output and
byte range; it supplies no initializer for a future load.

Actual saved owner IDs are `0..40, 62, 63`. The records do not expose outer Manager
requested IDs or ordinals. Consumers must match the actual Military cursor/range
and saved owner, never assume Player ID equals Manager position. Owner lifetime
IDs are scoped to one selected original Read or Write frame; this capture does not
prove continuous owner lifetime across streams, even where pointers or values
happen to agree.

## Validation limits

The actual rules/content/configuration/DLC/mod input identities are retained.
The gameplay database is in memory; its byte snapshot is unavailable. No SQL dump,
extra query, edit getter, synthetic receiver, policy call or candidate action is
added by this offline audit. The ordinary-call source profile has no runtime policy
call counters, so it does not prove a general absence of internal policy activity.

One actual 64-byte known-RNG observation is retained after Load. The frozen source
requires exact equality to that observation at each actual Save entry/return before
emitting successful markers. Independent raw endpoint dumps were not emitted;
the report labels those comparisons as source-guard witnesses. Complete gameplay
RNG equivalence is not established.

The classifier also runs 13 in-memory corruption refusal checks covering missing
initialization/destruction, wrong stream/profile/owner, frame stack mismatches,
wrong count/initialized values, wrong output hash/range, worker presence and an
unclassified missing database snapshot. The closed artifacts are never modified.

Reproduce the offline classification:

```sh
python3 tools/linux/military_initialization_capture_audit.py \
  --directory ../c6-ghidra/evidence/linux-military-initialization/native-2 \
  --output evidence/linux/military-initialization-capture.json
```

The next acceptance boundary is the independently reviewed production capture-to-
typed-snapshot bridge and exact Core wire comparison. This report does not promote
its own self-consistent facts into accepted native snapshots.
