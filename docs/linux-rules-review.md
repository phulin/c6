# Independent Linux rules boundary review

The immutable Linux base rules boundary passes review for the pinned
`linux-base-native-generated-v1` fixture. This is data loading coverage, not the
complete exact Civ VI state/action/turn implementation. The full objective remains
open. Machine-readable checks and limitations are in
[rules-review.json](../evidence/linux/rules-review.json).

The review independently read both private SQLite pairs, queried every table and
cell storage class, ran the Rust Linux export comparison, and checked actual
module/content/source-view identities against the pinned gates, including actual
original module bytes. Both configuration
files match byte for byte; both gameplay files match byte for byte. Rust validates
all 396 tables and 25,246 rows, including complete schema/metadata, rowids, SQL
value types and real bits, against the native artifact fingerprints.

The original host constructs actual ConfigurationDatabase and GameplayDatabase
objects, calls their original Initialize methods, and exports the borrowed App
connections using original `Database::Connection::BackupTo`. The recorded adapter
source hashes match the inspected source. All 110 actual symlinks resolve to the
pinned Linux sources, with unchanged bytes; only the `schema` directory spelling
becomes `Schema`. Runtime preflight rejects unexpected mapped files and symlink
targets. This is an explicit fixture input; original application filesystem setup
has not been reproduced. The preflight does not create an operating-system lock
against concurrent modification of the private view or installed assets.

The actual artifacts contain NULL, INTEGER and TEXT values and 470 gameplay REAL
cells. Two gameplay rows have intrinsic rowid zero. They contain no BLOB, invalid
UTF-8 text, embedded NUL text, negative-zero REAL, negative rowids or rowid gaps.
Those extra codec boundaries have synthetic regression tests, which do not become
original Civ VI fixture coverage. Full-file hashing also gates physical bytes;
canonical SQL row order is explicit rowid or primary-key order and does not
establish native planner/scan-order equivalence.

One acceptance defect was reproduced: the initial Linux inspector accepted a
changed content profile after its configuration hash was recomputed, then could
label the result as the fixed base profile. The recorded genuine exports are not
affected: their complete module and source gates match. The inspector now pins the complete
module/content/source-view profile and exact audited native configuration
identity. Independent rerun rejects that same modified report. All four focused
inspector tests pass, and reinspection of both genuine exports preserves their
complete file/semantic fingerprints.

The review reran `cargo test -p c6-data --lib` (six passing, two artifact tests
ignored), then explicitly ran the ignored Linux export test with both actual
private directories (one passing). Historical Windows native execution was not
run. Original native execution was not repeated by this review. Both recorded
runtime reports have identical bytes, so independent executions rely on the
runner's provenance rather than differing report identities. The runtime owner
confirmed distinct native subprocesses (CLI sessions 6605 and 22633), each with
new exclusively created output files; output directories intentionally do not
change the fixture configuration identity.

Only immutable SQL data transport is accepted. Native custom functions/collations,
connection/planner state, engine DefinitionIDs and initialized cache semantics,
full XP2 component activation, selected session options, map/player state,
actions and turns remain separate unresolved work. Proprietary database files,
source bytes and raw disassembly remain outside this public repository.
