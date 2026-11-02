# Original Linux Save roundtrip differences

The c6 Rust implementation still covers isolated primitives. Original native
benchmark Core Load and repeatable Save now succeed, but the lower diagnostic
Save→Load→Save comparison has a reproducible byte difference. This investigation
must identify actual serialized fields and their native read/write behavior;
unknown bytes are not normalized or masked.

The current required scope is exact gameplay state and gameplay RNG, with external
policies choosing every player's decisions. Proven native AI planning internals
are outside that scope. An AI namespace or manager name alone does not prove an
exclusion: gameplay-affecting data and RNG stored in those classes remain required.
Until specific fields and uses are established, changed data stays unclassified.

The canonical private native outputs are both 80,657,462 bytes:

| Lower diagnostic input | Original output SHA-256 |
| --- | --- |
| Benchmark extracted Core stream | `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884` |
| Captured output from the first row | `85d858efc7f3bd405a3f2349c596d3cab8d36fbd04e8efbf1cdfe9cce40e87a8` |

Each result repeats across independent processes with successful original Load,
complete input consumption, two identical consecutive Saves and complete native
provider cleanup. The canonical outputs differ in 89,198 bytes across 31,695
spans. [The retained failing case](../evidence/linux/game-load-roundtrip-differences-1.json)
records their ranges. Equal coarse native readouts—turn 592, phase 0, local player
0 and false processing/pending predicates—do not establish equality of their
other state. The existing fixture still omits saved RequestLoad/Begin and complete
host postload finalization; no cause is attributed to those omissions yet.

The first observational profile maps original serializer owner entry/exit offsets.
[The static audit](../evidence/linux/save-normalization-trace-contracts.json)
checks 203 exact XP2 exported owner methods against the pinned Linux module and
matching DWARF. Every selected interface returns its original OStream reference
and takes the original OStream and const owner references. Forwarding wrappers
call the original method and read the genuine MemoryOStream offset before and
after it. They change no stream fields, vtables or game fields. Call-parent IDs
separate repeated owner instances; raw traces remain private.

Observed ranges are usable only after the instrumented native profile reproduces
the known uninstrumented output hash and native cleanup. Owner boundaries alone
prove neither field semantics nor normalization. A classification requires the
specific original serializer field, its original Load handling, and the use that
shows whether it is gameplay/RNG state or excluded policy planning state. Logical
state equivalence, branch restoration and external decision readiness remain
separate acceptance requirements.

Both initial forwarding profiles now pass native Load/Save and complete cleanup,
while preserving both known original Save fingerprints exactly:
[benchmark trace](../evidence/linux/save-normalization-native-benchmark-1.json)
and [captured-input trace](../evidence/linux/save-normalization-native-captured-1.json).
The original calls and real stream behavior remain in place. Complete traces and
per-call mappings are private and hash-anchored by their public reports.

[The benchmark owner map](../evidence/linux/save-normalization-owner-map-benchmark-1.json)
and [captured owner map](../evidence/linux/save-normalization-owner-map-captured-1.json)
locate **89,156 of the 89,198 changed bytes inside Trade::Graph**, under the native
Trade::Manager serializer. The graph spans stream offsets 71,027,800–71,497,438.
Differences occur both within 522 serialized Trade::Route records and elsewhere
in the graph. Individual record boundaries differ between the two profiles; each
mapping therefore uses its own actual native trace. The remaining 42 changed
bytes are currently covered only by the outer Game::Instance boundary and require
more specific owner evidence.

Trade gameplay data cannot be excluded as AI planning internals. Exact Graph/Route
read/write code and matching DWARF identify native hash-container iteration and
rebuilding as a concrete ordering lead. A keyed comparison of all actual edge,
vertex, route, yield and path values is still required, together with iteration
and tie-break use evidence. Neither logical equality nor harmless encoding order
has been established from these owner ranges.
