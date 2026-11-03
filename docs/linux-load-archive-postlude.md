# Linux loaded Unit/Formation archive lifecycle

Native Read produces tracked dirtiness. A fully loaded idle object cannot be
assigned an empty dirty set from its wire record. This source audit closes the
registration and explicit clearing boundaries, and identifies a bounded Unit28
mutation trace; the complete loaded-idle baseline remains unverified. Complete
gameplay state, actions and turns remain unimplemented.

Reproduce without original-code execution:

```sh
python3 tools/linux/load_archive_postlude_audit.py --output target/linux/load-archive-postlude.json
```

The [checked metadata](../evidence/linux/load-archive-postlude.json) gates XP2
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`,
fingerprints complete native method extents and checks existing closed live
observations. Raw disassembly belongs in
`../c6-ghidra/evidence/linux-load-archive-postlude/`. Existing raw archive and
Game Read evidence was reused; no native run or writer/schema redesign occurred.

## Registration is an ordered transition

Unit's archive is at object+8; Formation's archive is at object+0. Their
constructors supply `AutoVariable::GetSyncManager` (`0x2824770`), which dispatches
through the actual EngineUtility virtual slot `+0x108`. The named host
`EngineUtility::GetGameSyncManager` (`0x32b4e10`) returns the real
`FGameSyncManager` singleton. Actual runtime dispatch and singleton availability
still need attribution for a selected load; a name alone is not that witness.
All 73 Unit / 17 Formation wrapper registrations are established separately by
[the archive schema](linux-unit-archive.md).

Both explicit Read routines end by calling host `FAutoArchive::SetID`
(`0x3c470a6`): Unit call `0x2c23d54`, Formation call `0x293b6cf`.
This is already sync registration at the Read boundary, rather than a deferred
player-container operation. `SetID` performs this exact sequence:

1. Return immediately if the stored 64-bit identity equals the requested one.
2. If archive byte `+0x50` is true, call controller `+0x58` virtual slot `+8`
   to unregister the old archive, then clear that byte.
3. Store the new identity at archive `+0x60`.
4. If the controller exists and identity differs from **zero-extended**
   `0xffffffff`, call controller virtual slot 0 with this archive. Set byte
   `+0x50` only if its bool return is true.

The identity sentinel is not `u64::MAX`. Logical wrapper attachment, header
identity and controller acceptance are distinct facts. A tracked setter uses
its wrapper's archive owner; successful global sync registration is not a
substitute for that owner. `SetID` and the concrete host sync registration body
contain no direct `clearDelta` call. The latter inserts into its actual identity
map when accepted, and checks identity collisions; admission cannot be replaced
with five true `*_archive_bound` inputs.

## Read state and idle state

Unit constructor explicitly edits components 62,63,64,65,68,66,67 while binding
nested component owners. Hero is edited too but is unregistered. Unit Read
`0x2c231f0` first calls gameplay `Unit::Instance::Reset` (`0x2c23dc0`).
Its unconditional top-level tracked edits are, in call order:

```
19,19,20,20,21,21,22,22,23,23,24,24,25,26,62,63,64,65,66,67,68,71
```

Reset additionally repeats index71 for each XP2 operation definition with a
resolved valid ordinary operation index. It writes that definition's actual
IsDisabled value. Nested components have their own Reset effects; Hero still
has no archive index. Reset never establishes clean dirtiness.

Successful current Unit28 Read then makes these explicit top-level edits:

```
31,34,35,36,37,39,40,47,49,54,55,57,57,
19,19,20,20,21,22,23,24,21,22,23,24,25,26,8
```

These calls happen even when their destination values are unchanged. Current
Read preserves a Formation lookup result before reading subsequent fields,
clears the separate runtime pointer at `+0xbc8`, sets archive identity, then
dispatches virtual post-load slot `+0x28`. The verified concrete implementation
calls interface post-load (definition resolution, tracked type index1), then
virtual domain initialization (tracked index12). The resulting explicit-call
membership has 31 indices, recorded in the artifact. It is a source-backed
lower bound before intervening clears, not a universal complete dirty baseline:
constructor/nested application, registration callbacks and enclosing gameplay
post-load must also be accounted for. No Rust baseline is promoted from it.

Formation1 Read's local ordered tracked effects are `[4,13]`: domain edit, then
unconditional evaluating-operation=false. Other current saved fields go through
their actual wrapper readers. Owner resolution and SetID follow. The enclosing
Player Formations container Read adds occupied objects, invokes Read and restores
allocator links; it does not perform a direct dirty clear.

Enclosing Game Read (`0x2d7aaa0`) calls post-load on players in actual
GetWasEverAlive vector order at `0x2d7b2d3`, among other gameplay initialization.
Player post-load (`0x2b32be0`) also iterates Units: a nondead Unit with valid
location and zero sight entries calls `Sight::Manager::PolulateSight`
(`0x2c46af0`, caller `0x2b32d1d`). Rules/Barbarian/processor and other game
post-load work follows. These gameplay continuations must be executed or
proved irrelevant to the selected state before labeling an object loaded idle.
AI namespace names alone do not exclude effects.

The existing [closed conditional SkipTurn capture](../evidence/linux/skip-turn-state-execute-3.json)
has `before_a.has_deltas=true` and **32 Unit indices**. All 31 explicit Unit28
indices appear, plus index44 (`m_iMoveAfterAttacking`). This is a real nonempty
idle witness, not a mapping cross-check or an immediate-Read witness. The
historical Read branch for `3 <= version < 28` performs tracked index44
assignment at `0x2c236e1`; version28 takes the ordinary wrapper reader at
`0x2c236b3`. Historical input could therefore explain index44, but that source
version has not been attributed here. Later gameplay mutation could also explain
it. Do not impose this witness's index44 on a current-format canonical import.
The existing capture supplies no Formation dirty witness.

## Actual clearing boundary

A full static host/XP2 disassembly scan found these direct `clearDelta` callers:

| Caller | Call site | Relevant condition |
| --- | --- | --- |
| `FGameSyncManager::TriggerGameSyncComplete` (`0x39d96de`) | `0x39d98f7` | Real FireWire session from manager session ID must be nonnull; then visit every archive in its actual registered map. |
| `FAutoArchive::loadDelta` (`0x3c46036`) | `0x3c4604f` | Before applying an actual archive delta payload. This is distinct from explicit Unit/Formation Core Read. |

The replay audit checks both direct call targets. It does not claim exhaustive
indirect-call analysis. An unavailable or unregistered archive is not cleared by
the sync-map iteration. Completion then releases its data stream storage,
clears the in-sync-check byte, invokes session callbacks and resets per-player
sync processing. Those callbacks are additional boundaries.

`FAutoArchive::clearDelta` (`0x3c45d18`) first visits every dirty wrapper in
native pointer-tree order, invoking wrapper virtual `+0x38`, then destroys and
empties the dirty-pointer tree. Its wrapper callbacks must be resolved before
promoting a generic Rust clear to full lifecycle equivalence. Host
`FAutoArchive::reset` (`0x3c4600e`) instead iterates **all registered variables**
and calls virtual `+0x50`; it is neither dirty clearing nor Unit gameplay Reset.
Neither Save output nor getDeltaIndexes itself clears dirtiness.

## Implementable application and remaining observations

The owned adapter can already attach logical field identities and retain the
ordered explicit calls above. It should expose an **unavailable full baseline**
until the selected import boundary is established. The lifecycle is constructor
registration/owner binding → Reset and current field application → SetID and
actual controller outcome → concrete Unit post-load → enclosing game/player
post-load → any actual sync completion → chosen idle boundary. Union subsequent
setter effects with the admitted existing history; only an evidenced clear
transition may erase it. FormationRead `[4,13]` is a local trace, not an idle
baseline token. Keep codec, at-Read application and loaded-idle state separate.

Minimal missing facts for the native reference owner, without a new run in this
task:

- Attribute the source Save/configuration and actual Unit Read version. Capture
  the chosen Units/Formations at Read return, complete Game Read return and the
  selected idle boundary, preserving logical identity and collection order.
- Cross-check each actual archive vector against 73/17 wrapper pointers, obtain
  full original hasDeltas/getDeltaIndexes outputs, header ID, registration byte,
  controller pointer and concrete controller slot0/slot8 targets. Observe the
  bool registration outcome and real sync-map membership where applicable.
- Attribute EngineUtility slot+0x108 and singleton to that load. Observe actual
  sync completion between those boundaries, its real session presence and any
  archive clearing or callback mutation. Preserve nested-component dirtiness
  separately where relevant; top-level vector indices do not exhaust state.
- Record whether Player post-load populated sight and any selected Unit or
  Formation mutations in later rules/game post-load continuations. This is a
  gameplay-state dependency, not a save-writer acceptance requirement.

The raw Save→Load difference of **89,198 bytes** remains unclassified and
unmasked by this work. Exact live state, RNG, actions and complete turns remain
open; this audit creates no action acceptance or native parity claim.
