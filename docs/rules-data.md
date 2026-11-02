# Immutable Rust rules input

`c6-data` loads the two actual original-native XP2 SQLite exports described in
[rules-snapshot.md](rules-snapshot.md). It is an immutable data boundary preparing
future Rust mechanics. The current simulator still implements isolated primitives;
loading these rules does not initialize a game, identify engine DefinitionIDs,
implement an action, or establish save/turn equivalence.

The caller must declare the complete supported `SourceIdentity` before loading.
It includes the native configuration digest, executable/GameCore and dependency
identities, ruleset, enabled DLC/mod selection, explicit source recipe digest,
source-sequence identity and null localization. `xp2_source_identity()` exposes
that descriptor; `Rules::load(path, &declared)` rejects any differing declaration
before opening files. This fixture is an explicitly selected original-source
recipe; it is not native component activation or a general DLC/mod resolver.
Only the complete pinned exports are accepted, with full-file hashes as well as
all schema, metadata, row-identity and typed-row fingerprints checked against the
public native snapshot manifest. Partial or modified databases fail closed.

SQLite is a bundled, read-only cold-load dependency. The loader checks integrity,
rejects external journal/WAL state, uses a read transaction, then verifies the
file bytes did not change. Both connections are closed before returning. All
fields in all rows are owned Rust values: NULL, signed i64, binary64 bits, TEXT
bytes or BLOB bytes. TEXT can retain invalid UTF-8 and embedded NUL; asking for a
Rust string rejects invalid UTF-8 instead of replacing bytes. Unknown columns
and all `table_xinfo` metadata remain available. Complete `sqlite_schema` records
retain SQL text for indexes, triggers and views as well as tables.

The canonical stream matches `c6.sqlite-typed-rowid.v1` byte for byte. Its order
is schema creation rowid, then intrinsic table rowid or explicit WITHOUT ROWID
primary-key order. That fingerprint order does not establish native unordered
SQL scan order, definition repository order, or a GameCore numeric type index.
`RulesRowId` explicitly names a SQLite rowid. No engine DefinitionID conversion
is supplied. Virtual tables and tables shadowing every intrinsic rowid alias are
rejected. Native connection functions, collations, cache construction and query
planner state are outside this boundary. Unknown collations fail through SQLite.

`Database::table()` and `Table::row_by_id()` perform immutable indexed lookups.
`Table::text_index()` builds a borrowed index once and rejects duplicate, NULL,
non-TEXT and invalid UTF-8 keys with table/row/column diagnostics. Its subsequent
`get()` calls allocate nothing and execute no SQL. Public iteration follows the
stored deterministic vectors; hash-map iteration is never exposed.
`Rules::unit(UnitType)` offers an owned typed projection of actual `Units` fields
(cost, base moves, combat, ranged combat, range and maintenance). Every other
field remains available in its full raw row. These base values do not include
modifiers or any game-state effects and do not implement unit mechanics.

## Reproduce and validate

Private native exports are required; no game databases or row dumps are included
in the public repository.

```sh
cargo test -p c6-data
cargo test -p c6-data -- --ignored
cargo clippy -p c6-data --all-targets -- -D warnings
cargo run --release -p c6-data --example inspect_rules -- \
  ../c6-ghidra/evidence/rules/run-1 ../c6-ghidra/evidence/rules/run-2 \
  > /tmp/c6-data-original-parity.json
```

The default tests are generic SQLite codec/schema fixtures, not gameplay parity
tests. The explicitly ignored native test loads both real exports independently,
checks every manifest table and typed row stream, and compares all typed unit
projections against their original retained SQL rows. Missing private files make
that native test fail when requested; default fixture tests do not stand in for
native parity. The example reports a `UNIT_WARRIOR` projection taken from the
validated original export, both full fingerprint reports and benchmark scope.

Both original-native runs were loaded successfully: 78 configuration tables /
1,866 rows and 428 gameplay tables / 41,704 rows, including internal tables.
All 506 table metadata and typed-row fingerprints match the authoritative Python
manifest. Semantic hashes are configuration
`afac77d5a09935e4e0c31a7b2ed9365e13243d0f14958d7b8baf6e6e34301a27`
and gameplay
`9138f1690db2fa37e22aab9b803c7a121ad9192d7116dec863d08531c2e9377a`.
The bundled reader's SQLite version is reported separately; successful hash
comparison does not claim cross-version planner or native connection equivalence.

The example measures a single load and one million immutable `UnitType` lookups
with a checksum consumed through `black_box`. Its SQL payload byte count is exact
for retained TEXT/BLOB bytes only; it excludes numeric values, vector capacity,
indexes, allocator overhead and process memory. Use an external RSS measurement
for whole-process peak memory. Timings include only implemented data operations
and cannot support FPS, game throughput or full-simulation performance claims.

One release-build measurement in this environment loaded the complete pair in
170 ms and averaged 26.5 ns for one million repeated successful immutable unit
lookups. A separate single-load process peaked at 43,716 KiB RSS, including
SQLite, validation temporaries, the reporting example and allocator overhead;
this is not retained rules heap size. Retained SQL TEXT/BLOB payloads total
3,121,338 bytes. These warm-filesystem, single-process measurements are examples,
not latency guarantees or gameplay benchmarks.
