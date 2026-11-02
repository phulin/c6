# Binary profiles and validation scope

Exactness belongs to a fingerprinted binary and configuration profile. The supplied Windows XP2 FinalRelease GameCore is the existing behavioral baseline for the Rust primitives, Windows bootstrap and native rules exports. The supplied symbol-rich Linux XP2 GameCore is a distinct native execution and research profile. Its relative age and currentness have not been established; the provenance/version audit is ongoing. Linux may become the primary execution path after that audit. There is no Windows-only platform requirement.

| Profile | GameCore SHA-256 | Established role |
| --- | --- | --- |
| Windows XP2 FinalRelease | `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c` | Existing original-code baseline for bounded Rust comparisons, rules inputs and Wine bootstrap |
| Linux XP2 | `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` | Symbol/DWARF research and genuine native context lifecycle; separate gameplay/version validation required |

The intended result remains a complete exact state/action/turn library. Neither profile currently provides that result. Native lifecycle success, rules loading and primitive comparisons establish their declared boundaries only. Choosing Linux execution must preserve the full objective rather than silently reducing it to an easier subset.

Host executables, dependencies, rules, DLC/mod selection, options and saves also need independent identities. Steam manifest build numbers, including `15296837`, are metadata rather than proof of cross-platform binary or gameplay equivalence. Reference address overlap and platform-specific layouts do not establish release chronology by themselves.

## Proven layout difference, unresolved chronology

The supplied Linux `Unit::CombatModifiers` DWARF has a 132-byte layout: all 41 named fields match the pinned reference's Linux layout, and its native `SetFightWhileEmbarked` stores at offset `0x82`. The supplied Windows profile has the documented `NoBarbXPLimit` field at `0x81`, shifting following flags; fresh Windows instructions corroborate the store at `0x81`. The semantic effect name also relies on the reference's effect/reader/database analysis.

This is evidence of an incompatible layout. It does not prove that Linux is older, noncurrent or behaviorally equivalent elsewhere. Official depot/build provenance and targeted behavior comparisons must answer those separate questions. [Reference-reuse analysis](reference-reuse.md) and [hash-gated contracts](../evidence/linux/reference-reuse-contracts.json) retain the observations; chronological labels in the reference require independent verification. Large DWARF/disassembly artifacts remain private.

## Reuse and coverage gate

Use symbols and the pinned cru121 reference to locate likely functions, types and interface owners. Verify the chosen profile's actual receiver, calling convention, field offsets, integer semantics, dependencies and instruction path before implementing a provider or Rust operation. Compare executable behavior and all affected state against that profile's hash-gated native oracle. A matching name, layout, Steam build number or passing test on another profile is insufficient.

Record the profile and all configuration/fixture identities with each result. Preserve existing Windows tests while evaluating Linux; never transfer their coverage claims silently. If profiles prove equivalent for a particular operation, record the comparison and its limits. Keep unsupported paths explicit and do not substitute a mismatched build without saying so. Existing [rules-data parity](rules-data.md) validates the Windows-native exported rules fixture; it does not implement gameplay or establish Linux rules equivalence.
