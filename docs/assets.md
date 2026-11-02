# Original asset identities and schema diagnostics

The supplied `../c6-assets` tree contains original Windows binaries, gameplay and
configuration XML/SQL sources, DLC descriptors, three benchmark saves, and 31
SQLite map databases. It contains no initialized gameplay or configuration rules
database. These assets support further bootstrap work; they do not establish
an exact full state/action/turn simulator.

`evidence/assets/supplied-tree.json` records a reproducible hash inventory of all
3,761 files, 42 `.modinfo` descriptors, native binary identities, save identities,
and SQLite schema hashes/counts. SQLite discovery checks the file signature,
not its extension: all 31 detected databases are `.Civ6Map` files. Their map,
plot, city and game-configuration tables are not a gameplay rules cache.

Reproduce the compact evidence with:

```sh
python3 tools/assets/inventory.py ../c6-assets \
  --output /tmp/c6-assets-summary.json --compact
python3 tools/assets/inventory.py ../c6-assets \
  --output /tmp/c6-assets-detail.json --file-manifest /tmp/c6-asset-files.json
python3 -m unittest discover -s tools/assets -p 'test_*.py'
```

The tree identity hashes canonical JSON records containing each relative path,
byte length and SHA-256, sorted by path. It excludes absolute paths and timestamps.
It identifies the supplied files, **not** enabled DLC, selected ruleset, mods,
game options, seeds, or an initialized session. The complete report preserves
declared mod action order, criteria identifiers, load-order values, per-file
priorities and existence checks. It does not evaluate criteria or sort actions
into an asserted engine execution order. `.dep` files describe art dependencies;
they do not supply gameplay database load order.

## Genuine initialized-state candidates

| Asset | Bytes | SHA-256 |
| --- | ---: | --- |
| `Base/Assets/Benchmark/Saves/XP2Benchmark.Civ6Save` | 7,348,717 | `a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8` |
| `Base/Assets/Benchmark/Saves/AIBenchmark.Civ6Save` | 1,241,407 | `097ac9c8719f0e62ee67b1cfb6596cc27c7f07a26308b71396c9eda89876c61c` |
| `Base/Assets/Benchmark/Saves/GraphicsBenchmark.Civ6Save` | 2,299,518 | `f9cc2c82af76d37aabd8ff50db8ffd3408713258d1874c5b66169fc7a685e700` |

These are genuine save fixtures. Their names do not prove compatibility with the
supplied DLL, enabled content, or rules identities. No supported save loader,
configuration/header interpretation, native turn advancement, or simulator state
transport equivalence is established here. Original loading must validate those
identities and expose the affected game state before a turn comparison is valid.

## Original SQL schema phase

A concrete, limited database diagnostic is available:

```sh
python3 tools/assets/schema_bootstrap.py ../c6-assets \
  --output-dir /tmp/c6-original-sql-bootstrap
```

The tool verifies all seven SHA-256 identities in
`evidence/assets/sql-schema-sources.json` before creating files. It executes the
unmodified original SQL using host SQLite, then atomically publishes
`configuration.sqlite`, `gameplay.sqlite` and a detailed `manifest.json`.
Output must be outside both the public checkout and the original asset root;
existing output directories are refused. No proprietary SQL or derived database
is committed to this repository.

The diagnostic sequence is four configuration schema scripts in filename order,
then separately gameplay `01_GameplaySchema.sql`, `02_AddTriggers.sql`, and XP2
`Expansion2_Schema.sql`. This creates 78 configuration tables with zero rows and
420 gameplay tables with 530 rows authored by the SQL itself, including schema
navigation metadata. It omits the eight tables described in the base XML schema
files and **all XML/SQL content loading beyond those seven scripts**. It does
not supply initialized rules or a playable game. Filename order for configuration
is a diagnostic choice pending verification against the original host.

`02_AddTriggers.sql` creates `Types`/`Kinds` insert triggers calling `Make_Hash`.
The diagnostic does not substitute an implementation. Later content inserts need
the original registered function. Host SQLite execution does not prove equivalence
to the bundled `DatabaseDLL_FinalRelease.dll` SQLite version, settings, extensions,
or importer. The scripts and source gate can also feed the original native
`Database::Connection::ExecuteMultiple` bootstrap adapter; that execution needs
its own native result evidence.

XP2's original `.modinfo` places `Expansion2_Schema.sql` at priority 2 and
`Expansion2_RemoveData.xml` at priority 1 in `Expansion2Core`, with action
`LoadOrder=-100` and `GameCoreInUse=Expansion2` criteria. It subsequently declares
87 files in `Expansion2CoreContent` and nine in `Expansion2MajorContent`.
A correct full load additionally requires the base importer, all selected
content criteria, action tie ordering, XML operation semantics, and the original
hash function; this tool deliberately does not infer those from file names.

## Native host path evidence

The SHA-256-gated supplied `CivilizationVI.exe`
(`e7450823cc8e00468cff7b9d7b97c63140eae38ae1d774ba4efa437556c42d63`)
contains these UTF-16LE literals at **file offsets**, not RVAs:

| File offset | Literal |
| --- | --- |
| `0xfb22b0` | `../../../Base/Assets/Database/Configuration.sqlite` |
| `0xfb2320` | `../../../Base/Assets/Configuration/Data/` |
| `0xfb23c0` | `/Cache/DebugConfiguration.sqlite` |
| `0xfb2490` | `/Schema` |
| `0xfb2820` | `../../../Base/Assets/Database/Gameplay.sqlite` |
| `0xfb2880` | `../../../Base/Assets/Gameplay/Data/` |
| `0xfb28c8` | `/Cache/DebugGameplay.sqlite` |
| `0xfb2918` | `Schema` |

None of the named prebuilt/cache databases is present in the supplied tree.
The literals identify concrete paths for tracing the host's database creation
pipeline; a string alone does not establish branch behavior or loader ordering.
`Debug/` instead contains FireTuner panels and its plugin, without debug database
snapshots. Capture a genuinely initialized `DebugGameplay.sqlite`, matching
configuration cache, enabled DLC/mod list and complete options from the original
host, or reproduce and compare its import pipeline, before claiming a rules
fixture suitable for full-state parity.
