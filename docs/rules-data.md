# Immutable Rust rules input

`c6-data` loads immutable rules from an explicitly selected binary profile and
its complete pinned native export manifest. It prepares inputs for the intended
exact Linux state/action/turn library; it does not initialize gameplay or implement
actions. The simulator still covers isolated primitives. Windows-native export
comparisons below are historical evidence only and establish no Linux parity.

`RulesProfile::LinuxXp2Base` identifies the supplied Linux GameCore SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
It registers the genuine two-run native Linux base export in
[`rules-base-snapshot.json`](../evidence/linux/rules-base-snapshot.json), source
profile `linux-base-native-generated-v1`. Original Linux Bootstrap reads 110
unchanged supplied base files through the explicitly fingerprinted `Schema`
directory case view. The full mapping is bound by the native configuration hash.
No DLC/mod activation, selected XP2 gameplay session or native definition-cache
setup is established. This fixture never substitutes Windows files.

The caller declares the complete `SourceIdentity` before loading.
`Rules::load_profile(profile, path, &declared)` first selects the profile's
registered manifest and requires exact equality with its full source descriptor.
The descriptor pins native configuration, executable/GameCore and dependency
identities, ruleset, enabled DLC/mod selection, source recipe, source-input
identity and localization. The compatibility field `source_sequence_sha256`
records the Linux content-gate inventory digest; it does not establish an
execution sequence. Original Linux Bootstrap remains the order authority.
`profile.source_identity()` exposes only an available registered descriptor; it
does not validate files. The compatibility
`Rules::load()` resolves only an exact complete registered source descriptor
before file access. A GameCore hash alone cannot select an input profile: base
and activated content may use the same binary. It applies the same complete gate.
Unknown or unavailable profiles fail before opening files.
There is no generic caller-supplied manifest or platform fallback.

`RulesProfile::LinuxXp2ActivatedDiagnostic` registers the two genuine native
Linux XP2 exports in
[`rules-xp2-request-none-snapshot.json`](../evidence/linux/rules-xp2-request-none-snapshot.json),
source profile `linux-xp2-native-request-none-generated-v1`. Original
`ConfigureContent` and `UpdateContent` select `RULESET_EXPANSION_2`, with the
manifest's actual enabled mod, active components, native getter order and
`en_US` localization. Getter order does not establish component execution order.
The source identity pins this diagnostic context independently of the base
profile despite their identical GameCore hashes.

Its native callback uses a genuinely constructed `LoadGameViewState` with
request `None` and constructor-default censorship `false`; saved `Begin` is
not called. The genuine UI owner is constructed but graphics initialization is
not established. These choices are profile limits, not assertions that saved
`Begin` policy or a complete initialized game would produce identical inputs.
There is no saved-game, native DefinitionID, action or turn equivalence claim.

`RulesProfile::WindowsXp2Historical` retains the original Windows fixture and
all 506 historical table fingerprints. The compatibility `xp2_source_identity()`
returns that historical descriptor only. A successful Windows comparison does
not validate Linux's native rules, object/cache layouts or gameplay behavior.

Complete registered exports require full-file hashes as well as every schema,
metadata, row-identity and typed-row fingerprint. Partial or modified databases
fail closed. The current historical fixture uses explicitly selected original
sources, not native component activation or a general DLC/mod resolver.

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
cargo clippy -p c6-data --all-targets -- -D warnings
C6_LINUX_RULES_RUN_1=/absolute/path/to/linux-rules/case-run-1 \
C6_LINUX_RULES_RUN_2=/absolute/path/to/linux-rules/case-run-2 \
  cargo test -p c6-data linux_native_exports -- --ignored
cargo run --release -p c6-data --example inspect_rules -- \
  linux-xp2-base ../c6-ghidra/evidence/linux-rules/case-run-1 \
  ../c6-ghidra/evidence/linux-rules/case-run-2 \
  > /tmp/c6-data-linux-native-parity.json
```

The default tests are generic SQLite codec/schema fixtures and profile-gate
tests, not gameplay parity tests. The explicitly requested Linux test loads both
real Linux exports, verifies every pinned table/row digest and compares all typed
unit projections to retained original rows. It requires absolute environment
paths because Cargo executes unit tests from the crate directory. Missing
private files make the native test fail when requested. Default fixtures do not
stand in for native parity. The ignored historical Windows test remains separate
and was not rerun for Linux acceptance. The example reports a `UNIT_WARRIOR` projection taken from the
validated original export, both full fingerprint reports and benchmark scope.

## Linux-native base validation

Rust loaded both genuine Linux exports independently: 78 configuration tables /
1,280 rows and 318 gameplay tables / 23,966 rows, including internal tables.
All 396 table metadata and typed-row fingerprints match the authoritative Python
manifest and both original file hashes. Native configuration identity is
`499dc9856ba562523ec70268224729232f751663988e44f4f9b0d6208e7f9328`.
Semantic hashes are configuration
`605f70d914c50f3533f3ac7895de24630fa5915981e0044011b4a094483b8cba`
and gameplay
`bea6bcabbd1905c8e498f6588005514d1c6f0bef0714c174d3b97e2f65710c22`.
This proves the declared base rules input boundary, not gameplay or turn parity.

Actual Linux-copy checks rejected a Units INTEGER-to-BLOB mutation, intrinsic
rowid change, added unknown column and a physical-only trailing byte. Source
identity mutations in each of the five descriptor fields failed before file I/O.
The untouched two-export comparison remained equal to live Python inspection.

One Linux-data release measurement loaded the pair in 115 ms and averaged
26.6 ns for one million repeated successful immutable unit lookups. A separate
single-load reporting process peaked at 30,156 KiB RSS. Retained SQL TEXT/BLOB
payloads total 1,855,407 bytes; this excludes numeric fields, indexes, vector
capacity and allocator overhead. RSS includes SQLite, validation temporaries,
the reporting example and allocator overhead. These warm-filesystem measurements
cover data only, not game startup, state, actions, modifiers or turns.


## Linux-native activated diagnostic validation

Rust loaded both private
`../c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-{1,2}/`
exports: 78 configuration tables / 1,865 rows and 428 gameplay tables / 41,700
rows. All 506 table fingerprints, complete schemas, row identities, typed-cell
hashes and original file hashes match the independent Python manifest. Direct
comparison also checks every retained schema value, column metadata value,
rowid and typed cell between the two Rust loads. Native configuration is
`4c392cbe97c20bed07617686d03260417dc2abe65aace7ce15988279279b43a6`.
Semantic hashes are configuration
`fc975853cdee7c04bf9b1a3a3a1cfe7835599c0a1aecd92d7566dac97d36a266`
and gameplay
`a0744270acd16fce0a545034bd8d8d41421aa2b2950efd1cbe9781c0fae6c9b0`.

```sh
C6_LINUX_XP2_RULES_RUN_1=/absolute/path/to/mod-activation-xp2-constructor-final-1 \
C6_LINUX_XP2_RULES_RUN_2=/absolute/path/to/mod-activation-xp2-constructor-final-2 \
  cargo test -p c6-data linux_activated_native_exports -- --ignored
cargo run --release -p c6-data --example inspect_rules -- \
  linux-xp2-activated-diagnostic \
  ../c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-1 \
  ../c6-ghidra/evidence/linux-rules/mod-activation-xp2-constructor-final-2
```

The native tests reject correctly declared base input against activated files
and activated input against base files. Default tests reject each other
profile's descriptor before file access and reject mutations of all five
source-descriptor fields while retaining the module hash. Copies of actual
activated files reject an INTEGER-to-BLOB change, intrinsic rowid change,
added column and physical-only trailing byte. Original private files remain
unchanged. Missing native files fail an explicitly requested native test.

One release measurement loaded the activated pair in 174 ms and averaged
27.6 ns for one million successful immutable unit lookups. A separate single-load
reporting process peaked at 43,860 KiB RSS. Retained SQL TEXT/BLOB payloads total
3,117,180 bytes. Payload excludes numeric fields, indexes, vector capacity and
allocator overhead; RSS includes SQLite, validation temporaries, the reporting
process and allocator overhead. These warm-filesystem measurements cover rules
data only, excluding game startup, state, modifiers, actions and turns.

## Historical Windows-native validation

Both Windows original-native runs were previously loaded successfully: 78 configuration tables /
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

One historical Windows-data release-build measurement loaded the complete pair in
170 ms and averaged 26.5 ns for one million repeated successful immutable unit
lookups. A separate single-load process peaked at 43,716 KiB RSS, including
SQLite, validation temporaries, the reporting example and allocator overhead;
this is not retained rules heap size. Retained SQL TEXT/BLOB payloads total
3,121,338 bytes. These warm-filesystem, single-process measurements are examples,
not latency guarantees or gameplay benchmarks.
