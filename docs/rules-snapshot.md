# Native rules database snapshots

The XP2 content fixture now preserves real configuration and gameplay databases
created by the original SQL/XML importer and GameCore embedded database writer.
Two independent native builds produced identical files and typed semantic hashes.
This advances rules-data availability; it does not establish full game state,
action, save, or turn equivalence. Native component activation is still bypassed
by the explicit, fingerprinted source recipe.

| Database | Tables | Rows, including internal tables | Bytes | File SHA-256 |
| --- | ---: | ---: | ---: | --- |
| Configuration | 78 | 1,866 | 815,104 | `7261acd9c163db38998134104ab1f69e58f177c4cb4a079618c4ba8e279fb101` |
| Gameplay | 428 | 41,704 | 8,736,768 | `b92d4ce5baf71a03349ec278f0804cc67faaf3559ed9579e62533757b846a7e6` |

The private files are under `../c6-ghidra/evidence/rules/run-{1,2}/`. No database
contents are committed publicly. The compact public
[`xp2-rules-snapshot.json`](../evidence/assets/xp2-rules-snapshot.json) records the
module identities, explicit XP2-only package selection, source recipe identity,
two native report hashes, file hashes, and per-table fingerprints. The fixture
configuration identity is
`5cd256b7b5b5c0cb54b3460b26972af98923dd7c7e0d5aa90e0463964d8b9002`.
The reports retain every input-source digest and native adapter source identity.

These snapshots precede localization activation and StartupGame. Their language
identity explicitly records `localization=null`, and they contain no localized
text database. Localization keys stored in rules rows are preserved as data.
They do not imply that a session language has been configured.

## Reproduce the native files

Use new private destination directories. The exporter refuses existing files.
The commands use the shared Wine prefix and should run sequentially with other
native probes:

```sh
python3 tools/bootstrap/run.py --content-recipe xp2 \
  --rules-snapshot-dir ../c6-ghidra/evidence/rules/rebuild-1 \
  --output /tmp/c6-native-rules-1.json
python3 tools/bootstrap/run.py --content-recipe xp2 \
  --rules-snapshot-dir ../c6-ghidra/evidence/rules/rebuild-2 \
  --output /tmp/c6-native-rules-2.json
```

The runtime imports all 213 pinned original sources, runs the actual GameCore
embedded writer, and completes native transactions before export. It backs up
both live writable keepalive connections before context initialization and
teardown. The native exporter independently gates DatabaseDLL SHA-256,
connection vtable RVA `0xb8578`, and copy method RVA `0x2670`.

The original method is `destination.CopyFrom(source)` at vtable byte slot
`+0xc0`. It obtains the source's raw SQLite connection through slot `+0x130`,
calls the bundled backup API with `main` as both database names, copies all pages
with step argument `-1`, finishes, and checks the destination error code. The
exporter uses original `CreateConnection` with read/write-create flags `6` for
the new file. Native source/destination counts are compared for every table,
then the original destination connection is destroyed to flush/close the file.
No Python-generated database, replacement XML parser, or guessed `VACUUM INTO`
path is involved.

Reproduce the bounded private backup ABI evidence with:

```sh
python3 tools/assets/backup_evidence.py \
  ../c6-assets/Base/Binaries/Win64Steam/DatabaseDLL_FinalRelease.dll \
  --output-dir ../c6-ghidra/evidence/rules/backup-api
```

## Fingerprint and compare

```sh
python3 tools/assets/rules_snapshot.py \
  ../c6-ghidra/evidence/rules/run-1 \
  --native-evidence evidence/bootstrap/native-xp2-rules-snapshot-1.json \
  --compare-directory ../c6-ghidra/evidence/rules/run-2 \
  --compare-native-evidence evidence/bootstrap/native-xp2-rules-snapshot-2.json \
  --output /tmp/c6-rules-comparison.json
python3 -m unittest discover -s tools/assets -p 'test_*.py'
```

The inspector opens files read-only, forbids journal/WAL side state, runs an
integrity check, reads within a transaction and verifies that file bytes did not
change. It requires successful native backup events, all 213 successful original
imports, the established module identities, matching runtime artifact hashes and
sizes, and exact native table/row counts. Comparison additionally requires the
same native fixture configuration identity. Per-table row checks are native;
semantic inspection uses the recorded host SQLite version.

The semantic encoding is `c6.sqlite-typed-rowid.v1`. Every row is encoded as byte
`A`, an unsigned 64-bit big-endian field count, then typed fields:

| SQL value | Encoding |
| --- | --- |
| NULL | `N` |
| INTEGER | `I` plus signed 64-bit big-endian value |
| REAL | `R` plus the returned IEEE-754 binary64 bits, big endian |
| TEXT | `T`, unsigned 64-bit byte length, original UTF-8 bytes |
| BLOB | `B`, unsigned 64-bit byte length, original bytes |

TEXT is read as a distinct byte wrapper, preserving invalid UTF-8 and embedded
NUL without conflating it with BLOB. Integer/real values remain distinct, and
binary64 encoding does not normalize negative zero or other returned bit patterns.
This fingerprints SQL values returned by SQLite; the separate full-file digest
preserves physical storage representation and header/page bytes.

Each row-stream digest starts with the encoding identifier and a NUL byte.
Schema rows are `(rowid,type,name,tbl_name,sql)` from `sqlite_schema` in schema
rowid order, including indexes and triggers. Ordinary table rows begin with the
intrinsic rowid, ordered by that rowid; the inspector selects an unshadowed
`rowid`, `_rowid_`, or `oid` alias. It refuses tables where every alias is
shadowed. WITHOUT ROWID tables retain primary-key columns and use their explicit
primary-key order. Virtual table extension state is unsupported and refused.
Table column metadata, schema creation rowids and persistent application/version/
encoding pragmas are also hashed. The database semantic identity hashes canonical
JSON containing those digests and identity descriptions, with sorted keys,
compact separators and UTF-8 encoding.

Runtime custom collations/functions, native connection pragmas, allocation state,
prepared statements and planner state are outside the serialized file. Native
`Make_Hash` is evidenced by the build, not reimplemented by the inspector.
WITHOUT ROWID ordering requires collations understood by the recorded inspector
SQLite; an unavailable collation fails inspection. No equivalence is claimed for
planner choices, physical btree scan order without ORDER BY, or behavior across
SQLite versions. Physical root-page layout is covered by the file digest rather
than the SQL semantic digest.

The typed semantic hashes are:

- Configuration: `afac77d5a09935e4e0c31a7b2ed9365e13243d0f14958d7b8baf6e6e34301a27`.
- Gameplay: `9138f1690db2fa37e22aab9b803c7a121ad9192d7116dec863d08531c2e9377a`.

These are reproducible immutable rules inputs for future loader work. They have
not been compared against an original application's initialized cache, and do
not themselves authorize advancing a Civ VI save or constitute a Rust rules
implementation.
