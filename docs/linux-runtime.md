# Native Linux context lifetime

The unmodified Linux `Civ6` executable can provide the real host symbols and static initialization needed by the unmodified `libGameCore_XP2.so`. `native/linux/host_probe.c` intercepts glibc `__libc_start_main`, forwards to the genuine function with the original initialization/finalization arguments and a replacement diagnostic main, then loads the complete GameCore with `RTLD_NOW`. No fake GameCore imports, reconstructed object layouts, ELF edits, Wine, window or display server are used.

The measured checkpoint creates and destroys three genuine original contexts, closes the GameCore handle and exits zero. This establishes a viable native oracle process, **not an initialized gameplay session or exact actions/turn library**. The original host application main is skipped. Original CRT and ELF initialization still run; host App initialization, Steam/network initialization through application main, and application shutdown do not run. The probe uses `_exit` after GameCore teardown because the skipped App startup does not establish application shutdown preconditions. A successful `dlclose` reports handle closure; shared GNU unique symbols or loader references may keep some mappings resident.

## Reproduce

A native C compiler and the original Linux assets are required. Ubuntu 24.04 also needs genuine GL/OpenAL loader dependencies. They may be installed normally, or extracted into the ignored work directory without installing a GUI:

```sh
mkdir -p target/linux/debs target/linux/sysroot
cd target/linux/debs
apt-get download libgl1 libglx0 libglvnd0 libopenal1 libopenal-data libsndio7.0
cd ../../..
python3 - <<'PY'
from pathlib import Path
import subprocess
for package in Path('target/linux/debs').glob('*.deb'):
    subprocess.run(['dpkg-deb', '-x', str(package), 'target/linux/sysroot'], check=True)
PY
python3 tools/linux/runtime_probe.py --output evidence/linux/runtime-context-lifetime.json
```

`--assets` and `--sysroot` override their defaults. The runner checks exact hashes of the original host, XP2 and all bundled host dependencies against `tools/linux/runtime-gate.json` before loader inspection or execution. It records all resolved immediate and transitive native dependencies, compiler, adapter source and compiled preload hashes in the configuration identity. `--expected-configuration HASH` refuses an identity mismatch before original execution. Original assets are never modified. The original text placeholders named `libc++.so` and `libtbb.so` are not used: required SONAME files `libc++.so.1` and `libtbb.so.2` are genuine ELF files already present.

The child runs in `target/linux/runtime`, with inherited `LD_*`/`C6_*` hooks removed, a 30-second timeout and no call to original application main. The output must exactly match all expected lifecycle markers as well as exit zero to pass this diagnostic. Missing dependencies, loader errors, signals and incomplete context cycles fail. No gameplay operation is implemented by this probe; rules, DLC selection, configuration databases, map, players, engine services, save loading, legal actions and turns remain outstanding.

## Evidence and ABI

[evidence/linux/runtime-context-lifetime.json](../evidence/linux/runtime-context-lifetime.json) contains the successful native run and the complete configuration identity. This is a distinct Linux target lineage and establishes no binary or behavior equivalence with the Windows DLL used by existing primitive comparisons.

The target host SHA-256 is `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`; XP2 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. The host is `ET_EXEC`, exports its symbols to the native loader and calls dynamically linked glibc `__libc_start_main`; GameCore has no `DT_NEEDED` entries. The host's genuine exports satisfy its undefined GameCore providers through normal dynamic resolution.

For this XP2 build, DWARF DIE `0x0cf610ad` identifies `DllCreateGameContext` at `0x2dcb980` as a no-argument function returning `GameCore::Interfaces::IContext1*`. The adapter preserves the pointer opaquely and passes it to `DllDestroyGameContext` at `0x2dcb990`; Linux x86-64 System V pointer calling conventions apply. The factory jumps to original `GameCore::Exports::Context::Create()` at `0x2c7c880`, which allocates and constructs the original context wrapper and registers its instance; destroy reaches original `Destroy(Context*)` at `0x2c7c8d0` for registry removal, destruction and deallocation. These addresses are evidence for this hash only. Raw target-derived ABI evidence stays private at `../c6-ghidra/evidence/linux/xp2-factory.txt` (SHA-256 `117df38a39a0e3bbce3f13eb17d534a2de774edadcd170528d9335fbb201b451`).
