# Independent consuming World review

This review covers `d0be715` (`LoadedState`), `2c0b781` (consuming
component split), and the offline candidate frozen at `3b733db`. It accepts
ownership and bounded saved-record assembly separately from live gameplay
initialization. Complete gameplay state, action execution, RNG transitions and
turns remain unimplemented. No original game execution is part of this review.

The source profile is the genuine `native-3` Read input
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
Its captured Military framing is admission evidence, not an implementation of
native Read. The separately captured current Write roles and other fixtures'
controls/WarTypes cannot silently become facts about this Read. Original-code
attribution remains pinned to the Linux XP2 module `a0b6611f…` and matching host
`106d4986…`, as documented in [the contextual import](linux-core-contextual-import.md).
OperationDef5 support is unrelated to assembling the admitted current-format
records and is not a gate here.

## Ownership and field coverage

`split_core` exhaustively destructures all 15 `CorePrefix` fields: the Map moves
to World, 13 fields move to `OwnedCore`, and the borrowed tail remains available
through the original owned image. `split_player` exhaustively handles all 46
`FirstPlayer` fields: Formations move to World, the Units manager is split, and
the other 44 fields remain owned. Of 39 Units-manager fields, its records move
to World and the other 38 remain retained. Religion, GlobalAi, Barbarian,
FreeCities and the serialized sync Random record are also retained. AI names
are not grounds for discarding records or classifying them as planning-only.

Unique input bytes move into `CoreImage::Unique`; immutable sharing is explicit
through `Arc`. Unit slot wrappers require one new vector, but each Unit value
and nested allocation moves rather than cloning. Formation vectors, member
order, paths and allocator buffers move directly. World builds separate dense
lookup indices without sorting native entry, layer, cell or reference vectors.
Existing World contracts preserve low16 lookup and requested/resolved IDs;
neither generation mismatch nor Formation membership is silently repaired.

`is_busy` reads the actual owned Unit's signed `i_operation_timer > 0`;
`can_move` reads its signed raw fixed-point `x_moves_remaining > 0`. They do not
use captured answers or runtime sidecars. Missing Manager/configuration/pause/
automation facts remain errors. The candidate does not initialize sidecars,
assert archive readiness, or execute an action. Saved flags remain retained
with saved provenance.

## Reproducible offline comparison

[The review harness](../tools/check/linux_loaded_world_review.py) instruments
the completed frozen candidate source and links against its already-built
Rust libraries. It performs a second direct `ThroughRandom` decode of the same
admitted image, then compares every retained Core/Player/Units field, complete
Map equality including layer/reference order, complete Unit and Formation
values, slot order and allocator metadata. It independently counts timer/moves
queries, checks low16 lookups borrow the owned records, and mutates one owned
Unit through six signed boundary values before restoring it. Neither decode
executes original code; this checks the ownership adapter against its existing
codec, not codec correctness against a new native oracle.

The second image/decode deliberately coexist with the assembled World. Review
process RSS and timing are therefore unsuitable as production measurements.
The original candidate's `assembly_seconds` measures combined decode and
assembly after admission, not assembly alone. Its `VmRSS` after assembly includes
retained immutable bytes, typed records, retained AI/components and lookup
indices. `VmHWM` is a process peak that also includes earlier artifact admission
and allocator retention. Neither measurement is per-action latency or exact
incremental World memory. Sidecar initialization is absent from that trial.

The original immutable single-decode run is now separately recorded in
[the startup evidence](../evidence/linux/loaded-world-assembly.json):
0.157825059 seconds for decode plus assembly; RSS 99,048 KiB before and
172,648 KiB after; process peak 963,456 KiB including admission. These are
measurements of that frozen binary and fixture, not of successor sidecars or
this review harness.

The independent comparison passed for all 43 Players, 1,262 Units, 1,250
Formations, 245 Cities and 1,284 Districts. Independent field counts are zero
busy Units and 705 Units with positive moves. All six signed mutation probes
passed. The current prefix consumes 71,027,796 bytes and 10,678,503 items,
retaining 197 Barbarian and 55 FreeCities instances and the sync Random record.
The remaining 9,629,666-byte tail has SHA-256
`645d2472c5f6bc1f4fe1460d706b4dc7acb346178a316669a6d528953ff0083c`.
[The public coverage record](../evidence/linux/loaded-world-review.json)
preserves the frozen source and review artifact identities. To reproduce using
the sole build owner's completed frozen artifacts:

```sh
python3 tools/check/linux_loaded_world_review.py \
  --source-root /tmp/c6-loaded-world-3b733db0dde5ee05c46a42b76bfb4d9c1e071e21/source \
  --deps /tmp/c6-loaded-world-3b733db0dde5ee05c46a42b76bfb4d9c1e071e21/source/target/release/deps \
  --manifest ../c6-ghidra/evidence/linux-military-initialization/native-3/candidate-paths-v2.json \
  --work-dir /tmp/c6-loaded-world-review-3b733db
```

## Lifecycle findings

The frozen adapter permits repeated `initialize_rosters`, replacing mutated
sidecars from saved arrays, and `bind_controls` unconditionally removes them.
`unit_mut` can also change type, Formation ID or saved arrays while
`unit_runtime` validates only owner/ID. Thus its identity guards alone do not
prevent stale derived fields. These are live-state lifecycle gaps, separate
from successful pure field queries and ownership assembly; the implementation
owner has been notified. A follow-up must preserve changed live arrays and
archive state, prevent accidental reinitialization, and make dependent raw
field mutation invalidate or explicitly update runtime views before advancing
actions.

The frozen candidate emits aggregate roster/query counts, but does not emit
the slot/Unit-ID list claimed by its documentation. Independent full-record
comparison supplies stronger adapter checks without treating those absent
output fields as verified evidence. Actual missing gameplay inputs remain
explicit; the unparsed tail and historical 89,198-byte Save→Load difference
remain unclassified and unmasked. Save production is not an action gate.

## Lifecycle successor

The independent follow-up checks `2ec623f` and `40c9775` through committed Core
`78698fb2b746a87b6f7287ac063faf20719bd2e9`. Unit and Formation initialization
are independent one-shot stages. Neither needs unrelated Manager slot/control
facts. First control binding retains prepared sidecars; subsequent binding or
stage repetition rejects rather than overwriting changed arrays or archive
state. Mutable access advances a revision, preventing later installation of
an initial observed snapshot. The witness's attributed source role is separate
from the unchanged decoder binding; the checks do not relabel this input Read.

Unit runtime getters check captured type, requested Formation ID and all seven
saved hash-array dependencies. They also resolve the current Formation and
compare owner/requested/resolved identity. The latter guard was added after
this review found that changing a Formation generation ID otherwise left a
Unit's cached resolved reference stale. Ordinary timer/activity/queue changes
do not reset live runtime arrays. Ownership/identity changes still fail through
the established container and archive checks.

The extended independent harness passed on all 1,262 Units and 1,250
Formations. It tests repeated initialization and control binding, preservation
of changed live arrays/dirty calls/evaluating state, raw type/Formation/hash-array
mutation refusal, the referenced-Formation identity regression, and witness
replay/after-advance refusal. Full retained/map/ordered-record comparisons and
the six signed scalar probes still pass. These lifecycle tests deliberately
use **synthetic definitions and controls**, exercising guard behavior rather
than asserting authentic native initialization. Their public evidence is
[separate from the frozen result](../evidence/linux/loaded-world-lifecycle-review.json).
Reproduction uses the same Python tool with the committed successor archive,
its completed dependency directory, and
`--lifecycle tools/check/linux_loaded_world_lifecycle_review.rs`. When a shared
dependency directory contains several builds, pass `--extern-artifact NAME=PATH`
for the exact `c6_core` and `serde_json` libraries; the evidence records their
tested hashes. The tool refuses ambiguous implicit library selection.

The genuine admitted rules-backed 1,262 Unit/1,250 Formation stage trial is
separately recorded by `48a60d8` in
[the initialization evidence](../evidence/linux/unit-initialization-definitions-check.json).
It does not compare omitted native unsaved state. Neither result establishes
complete native Read, complete executable gameplay World, actions, RNG
transitions or turns. The original frozen lifecycle findings and startup
measurements above remain unchanged historical results.
