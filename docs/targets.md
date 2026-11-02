# Linux target and historical profiles

The user directs **Linux-only execution and acceptance** for the remaining complete headless Civ VI state/action/turn library. All new native execution, mechanics validation, rules export and Rust parity work targets the supplied Linux XP2 profile. Existing Windows code and evidence are preserved as frozen historical work; do not continue Windows-specific tasks or native validation under this direction.

| Active artifact | Identity |
| --- | --- |
| `libGameCore_XP2.so` | SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` |
| `Civ6` native host | SHA-256 `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4` |
| Distribution metadata | Steam build `15296837`; Aspyr `1.0.12.9`, changelist `363760` |

No GNU build ID was reported for the principal host/GameCore ELF artifacts; content hashes are the identity gate. The [release audit](version-audit.md), observed 2026-11-02, matches all 20 installed depot manifest IDs to Valve's public branch. This supports the supplied installation's public-depot status, not cross-platform gameplay equivalence or verification of every copied file against Valve content. The [asset inventory](linux-assets.md) records module/dependency identities.

The complete goal is unchanged: exact state import/export and branching, external legal actions, full turn progression and deterministic RNG, all compared against the original Linux game. Native context lifecycle, localization and stream checks are bounded milestones, not a substitute for gameplay.

## Coverage isolation

The historical Windows XP2 FinalRelease SHA-256 is `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`. Its native primitive comparisons, rules exports and Rust `c6-data` parity remain valid only for their recorded Windows profiles. They do not certify Linux RNG, geometry, messages, rules or game state. Linux rules coverage requires an actual Linux-native export and independent parity checks.

Use symbols, DWARF and the pinned cru121 reference to identify likely types, ownership and functions, then verify the Linux receiver, ABI, offsets, integer behavior, dependencies and complete affected state. Gate execution by Linux host/GameCore, dependency, content, configuration and fixture identities. Preserve failures; unsupported or unavailable execution is not a parity pass.

## Layout difference without an age inference

The supplied Linux `Unit::CombatModifiers` has a 132-byte DWARF layout matching all 41 named reference fields; native `SetFightWhileEmbarked` writes offset `0x82`. The Windows profile has the documented `NoBarbXPLimit` field at `0x81`, shifting following flags, with its store corroborated by Windows instructions. This proves a layout difference, not that Linux is older or noncurrent. [Reference-reuse contracts](../evidence/linux/reference-reuse-contracts.json) retain the observations and their limits.

Never silently transfer behavior or coverage across profiles. Shared names, Steam build numbers and similar layouts are research evidence, not equivalence. Preserve historical work without running or expanding its Windows path; the active full-scope effort is Linux.
