# Independent Linux XP2 rules artifact review

The final Linux XP2 requestNone fixture passes immutable rules-data review across
506 tables and 43,565 rows. This is the specifically identified diagnostic callback
profile, not complete exact Civ VI state/action/turn behavior. The native target
is `libGameCore_XP2.so` SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.

The review independently checked all 2,779 original content inputs, 11 recorded
module identities, 21 filesystem inputs, the complete v3 launch layout and 15
adapter source hashes against actual bytes and the frozen native configuration
`4c392cbe97c20bed07617686d03260417dc2abe65aace7ce15988279279b43a6`.
Both preserved native runs have distinct empty-userdata/output directories,
exit zero, full framework/client cleanup, and identical effective selection.
Both actual SQLite pairs match byte for byte.

| Native database | Tables | Rows | Typed semantic SHA-256 |
| --- | ---: | ---: | --- |
| Configuration | 78 | 1,865 | `fc975853cdee7c04bf9b1a3a3a1cfe7835599c0a1aecd92d7566dac97d36a266` |
| Gameplay | 428 | 41,700 | `a0744270acd16fce0a545034bd8d8d41421aa2b2950efd1cbe9781c0fae6c9b0` |

Original App-owned connections are backed up through genuine Linux BackupTo,
with every native source/destination table count compared. Rust then gates full
file hashes, complete schema/column metadata, intrinsic rowids, cell storage
types and REAL bits. The focused Rust native test also compares every retained
schema/metadata/row/cell between runs and refuses copied artifacts changed in
cell type, rowid, DDL or physical bytes. Correct base identities refuse the XP2
files; both Linux profiles sharing a binary still require their complete
registered input descriptors before filesystem access.

Independent queries found NULL, INTEGER, TEXT and 680 gameplay REAL cells.
Gameplay includes three zero rowids and 60 tables with rowid gaps. Neither
artifact supplies BLOB, invalid UTF-8, embedded NUL, negative-zero REAL, negative
rowids or WITHOUT ROWID boundary fixtures; those codec cases retain their
separate synthetic coverage.

The real callback constructor at `0x2fd62d0` captures a genuine LoadGameViewState
owner. Its original constructor clears the word at owner+0x4b0, including the
censorship byte at +0x4b1. Request remains None; saved Request/Begin is not called.
The manifest preserves default censorship=false and the real UI constructor's
lack of graphics initialization. No equivalence to saved Begin or a different
censorship policy is assumed.

Actual native getters report one enabled mod after ConfigureContent, then one
enabled mod, one active mod and 46 active components after UpdateContent. Handles
are resolved through the real Modding database, retaining getter order. This is
not asserted to be component execution order. SQL rowids and mod handles are not
engine DefinitionIDs; a native getter comparison is still required for any such
mapping.

Review found two report acceptance checks to tighten: an arbitrary active module
path with the same XP2 filename was accepted, as was a false client-cleanup flag.
The inspector now requires the exact declared launch-module path and both
cleanup flags, plus 17 unique ordered ownership markers. Independent retesting
refuses both mutations while both genuine exports still pass. Details are in
[xp2-rules-review.json](../evidence/linux/xp2-rules-review.json).

Focused validation passed all eight XP2 inspector test methods, the genuine two-run Python
comparison, the Rust activated native export/mutation test, and nine independently
changed configurations with recomputed hashes. No original native execution was
repeated by this review. Proprietary databases and raw disassembly remain private;
public artifacts contain identities, fingerprints and bounded metadata.

Native connection functions/collations, planner state, localization database
bytes, initialized engine definitions, map/player/RNG state, saves, gameplay
actions and turns remain outside this accepted immutable-data boundary.
