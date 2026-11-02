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
not supply initialized rules or a playable game. Configuration filename ordering has now been recovered statically from the
original host; complete native XML import comparison remains outstanding.

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


## Complete base input recipe and original importer

`tools/assets/content_recipe.py` emits the complete base database source sequence
recovered from the supplied original executable. Reproduce the checked-in
`evidence/assets/base-content-recipe.json` with:

```sh
python3 tools/assets/content_recipe.py ../c6-assets \
  --output /tmp/c6-base-content-recipe.json
```

The sequence pins 111 inputs: four configuration SQL schema files, 20
configuration XML data files, five gameplay schema files (two SQL and three XML),
and 82 gameplay XML data files. It explicitly selects no DLC/mod IDs and labels
its ruleset `RULESET_STANDARD`; it does not represent the XP2 benchmark's settings.
The order is suitable for the next original-importer probe. It has not itself
been executed by this recipe generator and does not initialize a game.

The original configuration builder at EXE RVA `0xbc330` enumerates the schema
directory recursively, then `*.xml` in the complete data tree recursively. The
gameplay builder at `0xbf120` enumerates its schema directory without recursion,
then `*.xml` in the data tree recursively. Both sort schema/content vectors and
remove schema files from the content vector before loading schema, then content.
The filename callback `0xc09d0` replaces backslashes with slashes using
`0x7e91f0`. The comparator at `0x983b40` compares unsigned UTF-16 code units;
it preserves case. Recursive directory enumeration skips hidden/system entries.
The supplied source paths contain no such entries. Native enumeration and full
XML import still require a reproducible comparison before claiming database parity.

The shared dispatcher at `0xbd1f0` selects `.xml` (`0xbd320`) or `.sql`
(`0xbd2c0`). SQL reaches the original database connection's vtable slot `+0x28`
with length `-1`. The file buffer reader at `0xbd450` recognizes UTF-8 BOM bytes
`ef bb bf` and advances its content pointer by three bytes (`0xbd53f`–`0xbd568`).
This explains why directly executing raw `02_AddTriggers.sql` bytes differs from
the original file-import path: the native caller must apply the evidenced BOM
skip after verifying the raw input hash.

The original XML parser lives in the executable rather than a DatabaseDLL
export. Its outer importer constructor `0xbd160` stores the database pointer at
`+0`, calls `0x799910` on `this+8`, and calls `0x799d80` on `this+0x50` with the
database and a true flag. The XML wrapper calls `0x79d3a0` with parser pointer,
ANSI source filename, BOM-adjusted UTF-8 text, length `-1`, and callback object
`this+8` as the fifth argument. A zero return becomes success. Calling these
routines requires their actual allocator, imports, callback objects and globals;
mapping the executable for its empty-string getter does not initialize them.

Reproduce bounded private disassembly with:

```sh
python3 tools/assets/loader_evidence.py \
  ../c6-assets/Base/Binaries/Win64Steam/CivilizationVI.exe \
  --output-dir ../c6-ghidra/evidence/assets/original-loader
```

The evidence tool gates the executable hash and refuses raw disassembly output
inside this public checkout. The private directory includes a range/hash manifest.
The base recipe does not execute a replacement XML importer. Applying XP2 or any
other DLC still requires an explicit selected-content identity, original criteria
and load-order behavior, matching registered SQL functions, and a native database
comparison.

## XP2 benchmark saved content selection

The save's header key `8427ae5c` means required mods. It is not its enabled-mod
collection. Original `PopulateModMetaData` (`0x16baf0`) writes config accessors
`0x100520` to header key `88305ebb` and `0x100ea0` to `8427ae5c`.
Initializers `0x5390` and `0x5b90` identify their respective config collections
as `ENABLED_MODS` and `REQUIRED_MODS`. The original CRC path (`0x7a0da0` to
`0x7992c0`) starts at `ffffffff` without a final XOR, yielding config keys
`1b8cd1c8` and `fed47f44`. These differ from header metadata keys.

Both saved host-configuration copies explicitly contain those config collections.
Enabled collections at offsets 37,228 and 84,912 each contain only package
`4873eb62-8ccc-4574-b784-dda455e74e68` (XP2). Required collections at 39,778 and
87,422 contain the same package. Reproduce the compact evidence without
decompressing or advancing gameplay:

```sh
python3 tools/assets/config_mod_identity.py \
  ../c6-assets/Base/Assets/Benchmark/Saves/XP2Benchmark.Civ6Save \
  --output /tmp/c6-xp2-saved-mod-identity.json
```

[`xp2-saved-mod-identity.json`](../evidence/assets/xp2-saved-mod-identity.json)
records these saved identities and native evidence addresses. The saved mod
configuration supports an explicit XP2-only import candidate; it does not establish
that the current executable activates identical content or accepts this older save.

```sh
python3 tools/assets/content_recipe.py ../c6-assets \
  --xp2-saved-mods evidence/assets/xp2-saved-mod-identity.json \
  --output /tmp/c6-xp2-declared-content-recipe.json
```

The checked-in candidate pins 213 sources: the base 111, four XP2 frontend
configuration sources and 98 XP2 gameplay sources. It selects `Expansion2` as
game core. `Expansion2` criteria pass from `GameCoreInUse=Expansion2`;
`Expansion2AndBeyond` has `any=1`, so that same condition suffices without
inventing a playable leader set. No other installed DLC is implicitly enabled.

This extension orders actions by declared `LoadOrder` and uses declaration
order for action ties pending verification of actual component row-ID assignment.
Native component/settings file queries at RVAs `0x107aea0` and `0x107af40`
explicitly order files by `Priority DESC, Path ASC`; the recipe follows those
keys rather than declaration order. The component query is used at `0xb3a83b`.
The application action sorter (`0x25d0a0`) compares signed LoadOrder first,
unsigned dependency position second and signed component row ID third.
The component collector defaults missing LoadOrder to zero (`0xb34a60`).
Complete component row-ID assignment, dependency ordering and activation still
need original-host verification. The candidate explicitly records
`native_mod_load_order_verified=false`; it is not an exact activation recipe.
It contains only `UpdateDatabase` actions. Scripts, color/icon/art databases,
UI and other native service activation remain separate requirements. Native
import success alone will not prove the final database equals an original
initialized gameplay cache or establish state/action/turn parity.

## XP2 import phase failure and original correction

The native XP2 candidate initially failed at source index 134,
`Expansion1_Governments.xml`, with XML status 7 and a duplicate `Types.Type`.
The cause was a missing database phase transition in the diagnostic: base
gameplay content deliberately uses `foreign_keys=OFF`, whereas the original
mod database connection enables foreign keys (`0xb35e26`) and deferral
(`0xb35e3a`). Successful gameplay database setup also restores foreign keys at
`0xc0983`.

This matters to authentic source semantics. `Expansion2_RemoveData.xml` deletes
`KIND_GOVERNMENT`; the original schema declares `Types.Kind` with
`ON DELETE CASCADE`. Leaving foreign keys disabled prevents government type
removal, so later replacement rows conflict. Restoring the original connection
mode removes that failure without skipping, rewriting, or deduplicating rows.

Original `UpdateDatabase` handler `0x260150` wraps each action in a named
savepoint, using database slots `+0x90` and `+0x98` for savepoint/release. On
failure it calls `+0xa0` to roll back to the savepoint, then releases it.
Committing after each action is a different contract and prematurely checks
references intended to remain deferred across actions.

With the original foreign-key mode and action savepoints, all 213 source imports
return success. The current source-only candidate still fails its final native
commit with 1,734 foreign-key violations; that is a failed rules database build,
not initialized-game evidence. Many unresolved references are enum types used by
`DynamicModifiers`. The original host separately writes embedded GameCore data
through its wrapper `0x24de00`: it enables foreign keys and deferral, begins a
transaction, calls context export slot `+0x50` with the writable database, and
commits or rolls back.

For the pinned XP2 DLL, slot `+0x50` points to `0x590600`, forwarding the database
argument to underlying `0x45ed0`, which invokes writer `0x92a290` and
`0x979630`. This authentic phase is required before validating the mod rules
fixture. Its boolean return is insufficient evidence: the native method can
return true even when required prepared statements are absent. Compare its actual
`Types` and related data changes. The candidate recipe now identifies this phase
before source index 115, the first gameplay mod source.

[`xp2-mod-import-phase.json`](../evidence/assets/xp2-mod-import-phase.json)
preserves the original phase contracts, first failure and later failed-commit
evidence links. The original source sequence is retained. A passing native build
still requires the embedded writer and complete validation; full game state,
actions, save loading and turns remain separate boundaries.
