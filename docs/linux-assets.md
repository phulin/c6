# Linux asset identity and DWARF inventory

The supplied Linux distribution provides names, type layouts and native execution research: all three GameCore libraries contain DWARF and full symbol tables. Linux and Windows are distinct hash-gated target profiles; layout differences do not establish which gameplay revision is older. Their relative gameplay chronology remains unverified. Windows is the existing tested behavioral baseline. Linux may become the primary execution target after the version audit; each supported behavior still needs an oracle for its chosen profile. Neither profile currently establishes initialized-game or action/turn parity.

`evidence/linux/identity-assets.json` records SHA-256 identities, ELF dependencies, debug section sizes, dynamic symbol counts, interpreter and initialization tags for the supplied top-level ELFs. The supplied Aspyr JSON reports **1.0.12.9, changelist 363760**; the Steam app manifest reports **build 15296837**. These are distribution metadata, not proof of an internal gameplay version. The manifest hash is recorded without copying its Steam account identifier.

| Artifact | ELF role | Undefined dynamic symbols | Names also exported by Civ6 |
|---|---|---:|---:|
| Civ6 | ET_EXEC, non-PIE host, `/lib64/ld-linux-x86-64.so.2` interpreter | 870 | — |
| libGameCore_Base.so | ET_DYN, DWARF and symtab | 541 | 403 |
| libGameCore_XP1.so | ET_DYN, DWARF and symtab | 555 | 411 |
| libGameCore_XP2.so | ET_DYN, DWARF and symtab | 561 | 417 |

None of the three GameCore DSOs declares `DT_NEEDED`. This is not independence from the host: Civ6 exports 199,423 dynamic symbols. A matching exported name alone does not establish loader resolution, ABI correctness or a complete service implementation. XP2 has `DT_INIT` at `0x1dfe238` and a 1,776-byte initialization array (222 pointer slots); loaders execute initializers before returning from `dlopen`. The host has 401 initialization-array slots. See [native runtime evidence](linux-runtime.md) for the separate execution boundary when available.

The XP2 SHA-256 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; Civ6 is `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`. No GNU build ID was reported for these four principal artifacts. Use content hashes as the identity gate.

LLVM 18 DWARF statistics for XP2 report 252,565 function records, 252,426 with locations, and 248,683 inline records. These counts include compiler-generated/template entries and do not count distinct gameplay operations. Types include members, bases, source paths and parameter declarations; optimized local-variable coverage is incomplete. The compact [priority anchor evidence](../evidence/linux/identity-dwarf-anchors.json) records independently queried type definitions and function addresses. Prefer definition DIE offsets over generic names: `Instance` matches many namespaces and repeated declarations. In particular, gameplay `Unit::Instance` and `Cache::Unit::Instance` are different types.

Reproduce the inventory without loading or executing assets:

```sh
python3 tools/linux/inspect_elf.py inventory /home/phulin/c6-assets-linux \
  --output /tmp/linux-identity.json
python3 tools/linux/inspect_elf.py dwarf /home/phulin/c6-assets-linux/libGameCore_XP2.so \
  --sha256 a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa \
  --offset 0x06eeeb67 --depth 1 \
  --output /home/phulin/c6-ghidra/evidence/linux/xp2-context-definition.txt
```

The query tool requires LLVM `llvm-dwarfdump` (or `llvm-dwarfdump-18`), rejects an unexpected asset hash before querying, and supports exact `--name`, direct DIE `--offset`, and aggregate `--statistics`. `--name` scans the DWARF: LLVM accelerator `--find` returned no result for `Context` despite its presence. Keep raw proprietary DWARF, disassembly and projects in the private sibling `c6-ghidra` repository. The full private readelf report is `evidence/linux/full-elf-inventory.json` there; targeted evidence lives alongside it. Public files contain compact identity and selected interface facts only.

The recommended research path is to extract specific Context/Game/Player/City/Unit/script/processor interfaces, trace actual construction and serialization calls, and compare each affected boundary against an original-code oracle. A full unstripped Ghidra import may help later, but targeted DWARF queries already resolve sizes and member offsets without indexing hundreds of megabytes of debug data.

The supplied Steam build number matches the August 2024 build discussed by [cru121's reference](reference.md), while that reference describes its own Linux layout source as an older, pre-New Frontier Pass depot. That description does not date the supplied Linux binary. Neither observation establishes binary equivalence or relative gameplay chronology. The Windows oracle remains separately gated by DLL SHA-256 `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`. Linux addresses, layouts and ABI are separate evidence until each proposed mapping is verified. The assets also do not yet identify a loaded rules database, enabled DLC/mod set, or initialized-game configuration.
