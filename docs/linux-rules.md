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

The second command depends on the Linux services source-view hook. Two genuine populated native exports now pass and are recorded in
[`rules-base-snapshot.json`](../evidence/linux/rules-base-snapshot.json). A nonempty Units/Types/GameSpeeds set and populated
configuration database are mandatory acceptance gates; native Initialize's bool
alone is insufficient.

## Profile and fingerprint contract

The available base profile is `linux-base-native-generated-v1`, exposed to the
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
The inspector pins the audited base configuration `499dc985...` independently
of a report's recomputed self-hash, plus the complete module gate, exact source
inventory, Schema view and initialization-only service/session strategy. A changed
configuration or source strategy requires a separately audited profile; it cannot
be relabeled as this base fixture.

The source identity hashes the pinned Linux input gate; execution ordering is
owned by original Linux Bootstrap, not inferred from an inventory sort.

The database files and verbose inspection belong in the private evidence tree.
Only fingerprints and source/profile metadata belong in this public repository.
Native custom functions/collations and runtime planner state remain outside the
serialized SQLite artifacts; full startup/save/action/turn parity is unproven.


## Populated Linux base artifact

The source-view cause probe and independent repeat both succeeded, using actual
Linux Bootstrap and BackupTo methods. Their complete native configuration
identity is `499dc9856ba562523ec70268224729232f751663988e44f4f9b0d6208e7f9328`.
The source-view mapping identity is
`d9c4c18403af964a18c6df693038c96fe1c2db83618474c1d029740a7dbc1616`;
the full mapping is included in that native configuration identity.

| Native Linux base artifact | Tables | Rows | File SHA-256 | Typed semantic SHA-256 |
| --- | ---: | ---: | --- | --- |
| Configuration | 78 | 1,280 | `23dfe702e537d08230eec5ea44a0643cff789ebb438bbab0cb5ae24e9b9c8318` | `605f70d914c50f3533f3ac7895de24630fa5915981e0044011b4a094483b8cba` |
| Gameplay | 318 | 23,966 | `6e8b8384c91fbefa23896bb8b193f0e917420d92492d003ce66bfae4a31ae294` | `bea6bcabbd1905c8e498f6588005514d1c6f0bef0714c174d3b97e2f65710c22` |

Both native backups compare every source/destination table count. Both complete
files and typed schema/rowid/value fingerprints repeat identically. Private
files are in `../c6-ghidra/evidence/linux-rules/case-run-{1,2}/`. The native
reports are `runtime-rules-source-view-cause-probe.json` and
`runtime-rules-source-view-repeat.json` in `evidence/linux/`.

Reproduce the typed comparison with:

```sh
python3 tools/linux/rules_inspect.py \
  ../c6-ghidra/evidence/linux-rules/case-run-1 \
  --native-evidence evidence/linux/runtime-rules-source-view-cause-probe.json \
  --compare-directory ../c6-ghidra/evidence/linux-rules/case-run-2 \
  --compare-native-evidence evidence/linux/runtime-rules-source-view-repeat.json \
  --output /tmp/linux-base-rules-comparison.json
```

The Linux base fixture supplies immutable native-produced rules data for the
Rust loader. It has no activated DLC/mods or gameplay session and does not include
the full XP2 component pipeline or embedded definition setup. Genuine application
filesystem initialization is still distinct from the explicit Schema spelling
view. Native StartupGame or Load returning is not inferred from these artifacts.

Reproduce bounded Linux-only ABI disassembly privately with
`python3 tools/linux/rules_evidence.py`. No new Windows native execution is needed
for this workflow.

## Genuine ASL launch layout and full XP2 input gate

`ASL::ASL_GetAppAssetsPath` uses the executing host's `/proc/self/exe` parent
plus `/steamassets`. The supplied extraction instead has `Base` and `DLC` at
its root. This is separate from the Schema-view diagnostic above.
`tools/linux/rules_launch_fixture.py` stages a new private exact-byte host copy,
a real `steamassets` directory, and explicitly recorded input aliases:

| Fixture input | Original supplied input |
| --- | --- |
| `steamassets/base` | `Base` |
| `steamassets/dlc` | `DLC` |
| `aspyrassets` | `aspyrassets` |

A host symlink cannot substitute for the copy because `/proc/self/exe` resolves
to the original executable location. `validate(assets,fixture)` rejects a symlink
host, changed host bytes, changed aliases, undeclared entries or modified source
inputs. Original installed files remain unchanged. The native library search
path still uses the supplied libraries; none is replaced or synthesized.

```sh
python3 tools/linux/rules_launch_fixture.py \
  --destination ../c6-ghidra/evidence/linux-rules/asl-launch-1
python3 tools/linux/rules_inventory.py \
  --output /tmp/linux-content-inputs.json
```

[`rules-content-inputs.json`](../evidence/linux/rules-content-inputs.json) pins
2,779 supplied Linux Base/DLC files, 35 installed mod descriptors, and the exact
Base, XP1 and XP2 native module identities. The source inventory identity is
`4f5f1c64cd98cc6f6b39e2ac2a27a0b47f4292cef18e94732629cd188c7850b6`.
It is an input gate, not an enabled-mod list or asserted file execution order.
Descriptor declaration metadata remains evidence for original native resolution.
No imported Windows action order or module-transition claim is attached to it.

The launch mapping identity is
`528532714fca0f9562dd83df0071b47aecf666069b9781a54f10a81a2f3db1f2`.
The reusable validator hashes every gated content file and native module before
execution and returns these identities for inclusion in the runtime configuration.
The original ASL filesystem provider owns `ASL_FileInit`, path translation and
`ASL_FileQuit`; its independent lifecycle evidence is separate from this staging
tool. Full XP2 activation must use the genuine native framework's selected-mod,
criteria and action pipeline and capture actual Base-to-XP2 module transitions.
That full-content profile and its exported databases remain distinct from the
verified base-only fixture.
