# XP2 Map identity correction for the movement fixture

The first fact-only control returned 206 before producing a movement corpus. The
named diagnostic in `70ba982` then identified the failed condition: the live Map
vptr was module-relative `0x3afb160`; the probe incorrectly required base
`Map::Instance` point `0x3ad5360`. This was a probe contract defect. Both runs
compared zero movement costs. The original contracts and failed artifacts remain
preserved; no arithmetic mismatch or passing native comparison is inferred.

The functional correction requires the exact supplied Linux **XP2 Map** point
`0x3afb160`. It does not admit arbitrary subclasses or remove an identity check.
Existing [turn progression evidence](linux-turn-progression.md) already identifies
the actual XP2 Map table `0x3afb150`, including its inherited DoTurn binding.
The focused [movement audit](../tools/linux/movement_probe_audit.py) adds:

- XP2 Game::CreateMap (`0x2d77f20`) allocates the original object and calls
  XP2 Map constructor (`0x2d7b570`). That constructor calls the base with unchanged
  `this`, then stores table `0x3afb150` plus 16. The base subobject starts at zero.
- XP2 Map virtual `+0x78` remains original GetPlot (`0x2a48ba0`). No movement
  virtual target or direct inherited getter ABI changes.
- Plot default/copy constructors retain point `0x3ae23a0`. CityAttributes
  default/copy/move constructors retain point `0x3ae2360`. Their existing strict
  guards and called attribute virtual targets remain unchanged.
- Formation begins with its native archive subobject, whose constructor installs
  point `0x3ac7108`; it has no separate polymorphic Formation-class vtable.
  The probe records that archive identity. Manager instead has its initialized
  flag and checked ID-to-player vector extent; it has no class-vptr assumption.
- Exact constructor GLOB_DAT symbol/addend bindings are checked for XP2 Map,
  Plot, CityAttributes and Formation archive. The original host exports no
  competing dynamic symbols for those four vtables. Live identities still require
  the actual runtime attestations.

This revision also exports **original Terrain::IsWater** (`0x2bcadc0`, nine bytes)
as `is_water` in every live terrain registry row. The getter reads the native
definition flag and writes no state. It supplies a missing rules property for the
typed resolver; case-level Plot::IsWater answers are not substituted for rules.
This is explicitly a functional export addition, separate from the earlier
diagnostic-only change. Header `probe_contract_revision` is
`xp2-map-and-terrain-water-2`; the base JSONL schema stays v1 with additive metadata.

The immutable scalar Rust candidate remains SHA-256
`d4dddd41f8f157a951c5b43720e6a217fef1b6600439b307ee01e300c7ce6718`, reconstructed
from its `cb6113b` source manifest. It is neither rebuilt nor relabeled using the
new working tree. The separately frozen helper/source contract is
[movement-cost-probe-xp2-contract.json](../evidence/linux/movement-cost-probe-xp2-contract.json).
Strict C compilation, original method fingerprints, constructor relocations and
virtual bindings pass. A corrected genuine fact-only control must pass before
cost execution. No native run of this corrected helper has occurred at this
checkpoint; complete affected state/RNG, movement actions and turns remain open.
