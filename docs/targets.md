# Acceptance target and reference builds

The acceptance target is the supplied newer **Windows Gathering Storm XP2 FinalRelease GameCore**, SHA-256:

```text
324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c
```

The intended result is exact state, legal-action and complete-turn behavior for that binary with pinned rules, DLC/mod selection and configuration. The current Rust implementation and native bootstrap do not yet achieve that result. Running the Rust library on Linux does not change which original game build defines correctness.

| Artifact | Role |
| --- | --- |
| Supplied newer Windows XP2 FinalRelease DLL | Authoritative original-code acceptance oracle, with its own ABI, layouts and behavior |
| Supplied Windows host, dependencies, content and benchmark saves | Independently fingerprinted providers and fixtures; a recipe or extracted save stream alone is not a fully restored session |
| Supplied older Linux XP2 GameCore with symbols/DWARF | Reference for names, type families, ownership, call paths and behavior hypotheses; native checks establish facts about that Linux build only |
| Pinned cru121 reference | Index connecting symbol-bearing research to current Windows addresses; preserve mapping confidence and verify current target instructions |

Linux context initialization and teardown are useful research milestones. They do not replace the current Windows oracle or certify current Windows gameplay. The supplied Linux GameCore SHA-256 is `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; it remains a separate version and platform identity. Steam manifest build numbers, including `15296837`, are installation metadata rather than proof of cross-platform binary or gameplay equivalence.

## Proven version difference

The supplied Linux `Unit::CombatModifiers` DWARF has the older 132-byte layout: all 41 named fields match the reference's Linux layout, and its native `SetFightWhileEmbarked` stores at offset `0x82`. The current Windows layout has the documented `NoBarbXPLimit` insertion at `0x81`, shifting following flags; fresh current Windows instructions corroborate the store at `0x81`. The semantic effect name also relies on the reference's effect/reader/database analysis. This is a concrete incompatible layout, not a claim that every mechanic differs or that all other fields match.

[Reference-reuse analysis](reference-reuse.md#gamecore-version-audit-the-supplied-linux-layout-is-older-than-current-windows-here) and [hash-gated contracts](../evidence/linux/reference-reuse-contracts.json) retain the proof and its limits. Large DWARF/disassembly artifacts remain private.

## Reuse gate

Use Linux symbols and the pinned reference to find likely current Windows functions and interface owners. Then verify the actual current Windows receiver, calling convention, field offsets, integer semantics, dependencies and instruction path before implementing a provider or Rust operation. Compare executable behavior and all affected state against the hash-gated current Windows oracle. A matching name, old layout, Steam build number or passing older-Linux test is insufficient.

Record the target module, host/dependencies, rules and fixture identities with each result. Keep unsupported paths explicit. Do not substitute an easier older build for the requested target or expand a bounded passing test into full-game equivalence. Current [rules-data parity](rules-data.md) validates actual Windows-native exported rules inputs; it does not implement the owning gameplay systems.
