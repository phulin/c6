# Independent Linux definition registry review

The reviewed component remains an isolated byte-hash and lookup primitive. The
original six-Yield observation supports its bounded mapping for one loaded Linux
benchmark context. It does not establish exact state/action/turn behavior,
registered rules-export identity, arbitrary mods, collision gameplay, or Units IDs.

The review covers public checkpoints `bf149cf`, `efa7e59`, `d2b4348` and runner
integration `7b500b6`. Original engine execution and native rebuilding were not
performed during this review. Existing private captures were reused read-only.

## Semantics and successful evidence

Pinned Linux target bytes and retained disassembly agree with the Rust hash:
reflected polynomial `0xedb88320`, seed `0xffffffff`, unsigned byte input, first
NUL termination, case sensitivity, and no final xor. Native empty string and
lowercase boundaries agree with Rust. Embedded-NUL and high-byte behavior have
static evidence and synthetic Rust checks, rather than this native observer's
runtime coverage. `Yield::SetYieldType(nullptr)` storing zero is a separate
contract; the registry models supplied byte names, not nullable native objects.

Native `Find` compares its unsigned 32-bit argument against vector count before
hash lookup; small numeric keys select append-order entries. The hash comparator
uses unsigned order, so hashes above `INT32_MAX` remain valid. Native lower-bound
insertion places equal hashes before earlier insertions, and lookup returns the
first equal entry. Sorting Rust pairs by unsigned hash then descending insertion
ordinal preserves this precedence. Native `FindIndex` returns the stored index
bits, with all-one bits on a miss; Rust uses `Option<usize>` as an abstraction.
No native collision fixture was executed. Supplied vector order and rowids are
metadata inputs, not proof of a loaded collection or a rowid-to-index conversion.

Indices/counts require their bounded native domain: the builder stores the low
32 bits of vector size as an index. Rust holds `usize` ordinals and now rejects
collection counts outside the unsigned 32-bit domain before constructing the
hash indexes. It does not model oversized collections whose index assignment
wraps after `u32::MAX`. No such fixture, allocation, or overflow experiment was
performed. The observer separately
rejects zero or more than one million definitions; the accepted context has six.
Rust database rowids retain signed 64-bit database identity and do not claim the
native object's signed 32-bit field narrowing.

Independent static audit reproduced all 20 recorded method fingerprints, host CRC
fingerprint, and Yield SQL exactly. The audited native report reproduced exactly
from the retained runtime report and both full canonical captures. Runtime report
SHA-256, canonical configuration digest, observer source digest, and copied source
identity all matched the public report. The runtime proves framework and client
cleanup and active XP2 context. Both captures independently measured 80,657,462
bytes and SHA-256
`ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`.
This establishes their serialized boundary, not all unrecorded live memory.

The explicit report-driven Rust comparator passed all six mappings: indices
0–5, database rowids 1–6, native `MakeHash`, stored hashes and `FindIndex`. It also
matched the reported empty/lowercase/absent boundaries. This context has native
configuration digest
`c926ba2347ef6515da4b260f960957dab7389239b2cd59526407c72e74c7a1e4`.
Its rules snapshots are empty; native enabled-content selection is recorded.
There is no basis to equate it with the separately registered request-None rules
export merely because six Yield names/rowids agree.

## Acceptance checker findings

Two reproducible gate defects were reported to the implementation owner. These
defects concern acceptance of other reports; they do not invalidate the
independently verified original successful fixture.

1. The audit's alternate `exit_code` report path accepted `exit_code=0` and an
   XP2 module label while both framework/client cleanup flags were false. Reusing
   the real captures with that altered metadata generated a native-accepted
   report. Cleanup must be required for every native acceptance path.
2. The Rust native comparator accepted the successful report after replacing
   both host and configuration digests with 64 zeroes. Its initial checks only
   established the GameCore identity and a claimed unchanged-save boolean.
   A trusted audited-report handoff must validate the source/module/configuration
   provenance or explicitly require a separate fail-closed validator.

The implementation fixes are in commit `3a0d224`. The hardened audit now requires the supported original runtime envelope, strict
cleanup booleans, named module identities, matching static observer gate and
recorded capture identities. The Rust comparator requires the registered
benchmark configuration, runtime-report/canonical-capture provenance and
recomputes its complete source identity. Independent
retests reject both original exploits, misplaced digest keys and false/truthy
cleanup values, while the genuine fixture still passes. The historical successful
report remains unchanged; re-auditing it changes only the postprocessor source
digest. Final source identities and retest status are recorded in the accompanying
[review evidence](../evidence/linux/definition-ids-review.json).

The full raw Save→Load difference remains the recorded 89,198-byte failure;
this review does not classify or mask those bytes. Native collision/mod cases,
Units registry queries, rowid narrowing extremes and collection overflow remain
outside the accepted native coverage.
