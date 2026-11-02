# Original Linux ASL filesystem initialization

The replacement host main skips original ASL startup. Genuine `ASL::ASL_FileInit` establishes the original emulated-path prefix mappings and native suffix case conversion. The diagnostic calls that exact original initializer, observes the original public `ASL_EmuToNativePath` API and invokes original `ASL_FileQuit`. It provides no replacement path resolver or importer. Target-specific symbols, signatures and private evidence hashes are recorded in [filesystem-abi.json](../evidence/linux/filesystem-abi.json).

Original `ASL_GetAppAssetsPath` derives the executable parent from `/proc/self/exe` and appends `/steamassets`. The supplied asset tree places uppercase `Base` and `DLC` directly beside the executable and lacks that `steamassets` directory. Calling FileInit in the flattened tree alone therefore cannot establish the expected content root.

The isolated fixture copies the exact unchanged Linux executable into a private launch directory. It supplies a real `steamassets` directory with `base` and `dlc` links to the supplied original `Base` and `DLC` directories and an `aspyrassets` link to original metadata. Copying the host is necessary because `/proc/self/exe` resolves executable symlinks. Original content bytes remain unchanged. [The launch fixture tool](../tools/linux/rules_launch_fixture.py) validates the complete declared directory layout, copied host identity, 2,779 content input files and all three GameCore module hashes. The filesystem runner additionally gates all 21 ASL metadata inputs and records every resolved native dependency and adapter identity.

`libASL_Interpose.so` is already the first original host `DT_NEEDED` dependency and remains genuinely loaded; the diagnostic preload does not replace it. No launch shell script was present in the supplied tree. No Windows execution, binary patch or guessed Windows layout is involved.

## Reproduce the isolated boundary

Create a new private exact-byte launch fixture using `tools/linux/rules_launch_fixture.py`, then run:

```sh
python3 tools/linux/filesystem_probe.py \
  --launch-host /home/phulin/c6-ghidra/evidence/linux-rules/asl-launch-1/Civ6 \
  --output evidence/linux/filesystem-full-gated-init.json
```

The [fully gated native result](../evidence/linux/filesystem-full-gated-init.json) exits zero. Before FileInit, three emulated Base gameplay-schema, Base configuration-schema and XP2 modinfo paths do not exist. After FileInit, original normalization maps them to existing supplied lowercase files. A missing-path fixture remains missing. A second direct original FileInit call returns through its native guard. Original FileQuit removes its genuine generated temporary directory. [An expected-configuration mismatch](../evidence/linux/filesystem-configuration-refusal.json) is refused before original native execution.

Original ASL data-root discovery uses `getpwuid(getuid())->pw_dir` and appends `/.local/share/aspyr-media`; it does not use a substituted `HOME`. The fixture records that dependency and the directories created by original code. The isolated runner refuses existing user configuration files or undeclared directories in that root. The measured runs created only empty game and `Aspyr` directories. Original temporary roots are generated under `/tmp` and removed by original FileQuit. Original application main, preferences startup and application shutdown are omitted.

The adapter supports one ASL fixture lifetime per process. It rejects restart after FileQuit because original cached temporary-root strings are not reset by that cleanup. The process retains the diagnostic `_exit` boundary.

The [integrated original filesystem/rules lifecycle](../evidence/linux/runtime-filesystem-rules-lifecycle.json) additionally runs FileInit before loading GameCore, then original App/database initialization against the installed-root fixture without a Schema-only source view. Native backups contain 78 configuration tables with 1,280 rows and 318 gameplay tables with 23,966 rows; all original source/backup table counts match. Native Context and host-service destruction, `dlclose` and FileQuit complete with exit zero. This proves that original ASL case conversion can replace the Schema-only source view for this measured Base bootstrap profile.

This establishes the observed native filesystem and Base import boundary. It does not establish DLC/mod activation, saved-game configuration, a loaded game, legal actions or complete turns. The existing explicitly gated Schema source view remains preserved as a separate diagnostic fixture.
