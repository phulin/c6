# Linux native rules artifacts

The Linux execution profile is independent of historical Windows fixtures. Its
GameCore SHA-256 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`;
the host SHA-256 is `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
The objective remains complete exact state/action/turn behavior. This work
establishes database execution and artifact boundaries, not that full objective.

## Native backup boundary

`native/linux/rules_snapshot.c` obtains actual App-owned configuration/gameplay
connections through the exported `Civ6App` getters. It opens new destination
connections with original `Database::CreateConnection`, then calls the genuine
Linux `Database::Connection::BackupTo(source,destination)`. The runtime gates all
original modules and records the adapter source identity before execution.
Existing destinations are refused. Original SQLite APIs compare every table's
source/destination row count; destinations are closed through original Destroy.
No host-generated database or Windows vtable/address is used.

The Linux backup implementation at `0x395cb6c` forwards to `0x395cb7c`, which
obtains the destination SQLite connection and invokes original
`sqlite3_backup_init`, `sqlite3_backup_step(-1)` and `sqlite3_backup_finish`.
It returns whether the destination error code is zero. Native DB getter methods
at `0x303cd40` and `0x3041980` return borrowed connections; the exporter does not
destroy the App's source connections.

## Preserved empty-base failure

Two independent native base exports succeeded as backups but exposed a content
bootstrap failure: configuration has zero tables/rows; gameplay has eight tables
and zero rows. Original base database Initialize returned success despite absent
rules. Those snapshots are not registered as populated rules or definition
coverage. The snapshot inspector explicitly refuses them. Evidence is in
`evidence/linux/rules-base-native-{1,2}.json` and
[`rules-empty-base-refusal.json`](../evidence/linux/rules-empty-base-refusal.json).
Private database files remain in
`../c6-ghidra/evidence/linux-rules/base-run-{1,2}/`.

Fresh Linux instructions establish the gameplay bootstrap path:
`GameplayDatabase::Bootstrap` at `0x3041ab0` appends the literal `Schema` at
`0x3041b33`, enumerates that directory, sorts schema/data vectors and invokes
original DatabaseLoader. The supplied asset directory is spelled `schema`.
Recursive XML enumeration can create the eight XML-described schema tables while
SQL schema enumeration misses the directory. This explains the observed output;
original application filesystem-case initialization is not established yet.

`tools/linux/rules_stage.py` creates an explicitly identified private diagnostic
source view. It verifies all 110 Linux source hashes from the runtime content
gate, links unchanged original bytes and adapts only the directory spelling
`schema` to the Linux executable's `Schema` literal. It does not modify installed
assets, execute a replacement importer, or reuse a Windows source sequence.
The fixture's source mapping must be recorded when using this adaptation; it is
not proof that original application filesystem initialization ran.

```sh
python3 tools/linux/rules_stage.py \
  --destination ../c6-ghidra/evidence/linux-rules/source-view-1
python3 tools/linux/runtime_probe.py --initialize \
  --rules-source-view ../c6-ghidra/evidence/linux-rules/source-view-1 \
  --rules-snapshot-dir ../c6-ghidra/evidence/linux-rules/populated-base-run-1 \
  --output /tmp/linux-rules-base-native.json
```

The second command depends on the Linux services source-view hook. A genuine
populated native export and repeat comparison are required before publishing
`rules-base-snapshot.json`. A nonempty Units/Types/GameSpeeds set and populated
configuration database are mandatory acceptance gates; native Initialize's bool
alone is insufficient.

## Profile and fingerprint contract

The planned base profile is `linux-base-native-generated-v1`, exposed to the
Rust loader as a separate Linux base fixture. It selects no DLC/mod actions and
has no active gameplay session or language. Full Linux XP2 activation, original
component criteria/dependency/file ordering and embedded definition setup remain
separate required work. A current Linux library name does not turn base content
into full XP2 coverage.

`tools/linux/rules_inspect.py` accepts only Linux-gated native runtime evidence,
genuine successful backups, matching artifact hashes/sizes, exact native counts,
and populated base content. It reuses the platform-independent typed SQL/rowid
fingerprint encoding documented in [rules-snapshot.md](rules-snapshot.md), without
reusing Windows module or recipe gates. Two independent native builds must match.
The source identity hashes the pinned Linux input gate; execution ordering is
owned by original Linux Bootstrap, not inferred from an inventory sort.

The database files and verbose inspection belong in the private evidence tree.
Only fingerprints and source/profile metadata belong in this public repository.
Native custom functions/collations and runtime planner state remain outside the
serialized SQLite artifacts; full startup/save/action/turn parity is unproven.
