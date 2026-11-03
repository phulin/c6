# Independent first activity-report review

The first synchronous activity report in `9491031`, with the Plot-cost API from
`9744303`, is supported for the recorded `MemberNotReady` path. The independent
[review report](../evidence/linux/loaded-activity-reporting-independent-review.json)
rechecks all 266 pinned original method bodies and the relevant encoded native
branches. No original process was launched. Complete gameplay state, actions,
turns, affected-state comparison and gameplay RNG remain unfinished.

The retained Rust checker executable has SHA-256
`2667fe4b21b2ac68dc22e097c1cdb36bd9c30606571cb1c3c5a1b5580740496e`.
Its independent replay against the existing captures reproduced **all 23 result
fields** in the original first checkpoint. This is a replay of the retained
executable, not a source rebuild or a new native differential comparison.

At the first snapshot, actual owner 0 / Unit 15,794,187 is AWAKE, has 256 Q8 moves
and still has its old single MoveTo node. Native member readiness tests pending
operations at `0x2c281a4`, with the false branch at `0x2c281ab`. ReadyToMove's
false member result short-circuits at `0x2c28038`, before formation readiness and
CanCyclePast. The provider therefore correctly avoids the adjacency and
HoldCycling continuation for this report. The later captured old-empty queue
HasAdjacentMove=true call is separate evidence and was not used as this answer.

The snapshot query sees the authoritative activity setter's completed change
while the old queue remains present. The provider then copies identity/activity
into one pending event35 publication at the host translation boundary. The next
report is UnitOperationDeactivated `a60328c5`, where the first provider rejects
explicitly. The action prefix and the already prepared activity publication remain
intact. The replay also retains its negative shape, identity and unavailable
playback-drain checks. The provider's direct loaded method takes an immutable
world borrow, stores copied scalar identities and diagnostic observations, and
retains no actor/world reference. The Unit-only callback entry rejects it. The
shared loaded boundary must release the actor's mutable borrow before resolving
world queries; that integration was a frozen WIP dependency described below.

Omitting the concrete reflected Unit snapshot output and stock visual UnitFlag
dirty history is supported by their existing consumer/lifetime evidence.
Conditional gameplay query effects remain required. Empty dynamic properties are
checked on the actual current Unit; nonempty/custom copying rejects. OPERATION
facade projection and missing HoldCycling callbacks reject, and playback does not
silently drain. The stock/no-added-consumers scope remains a caller admission
condition; the supplied Lua inventory cannot independently establish that active
profile condition.

Static review of the unexercised adjacency implementation found no concrete
success-path defect in its stated bounded subset. Its neighbor ordering,
placement-before-HasMoved ordering and conditional Plot movement-cost comparison
follow the pinned source. Earlier unsupported neighbors stop the scan. Foreign
borders, embark/city-domain fallback, spy handling, nonempty defender cells and
barbarian continuation remain explicit unsupported dependencies. This first
fixture exercised **none** of adjacency, Plot movement cost or HoldCycling;
reviewing those source branches does not provide native boundary fixtures or
general placement/action acceptance. Errors may preserve completed prefix work
and provider diagnostic observations; they do not promise rollback.

The historical source identity has a material reconstruction limit. The original
metadata honestly records `immutable_source_snapshot=true` and
`root_changed_after_snapshot=true`. Of its **167 source hashes, 165 match** the
files retained by commit `9491031`. Its `loaded_actions.rs` preimage differs from
that commit, and `loaded_actions/handlers.rs` was absent from the commit. In
particular, the committed shared module did not yet contain the reporting export
and `send_loaded` integration used by the retained executable. The first temporary
source directory was deleted, so hashes and the binary cannot recover those
missing source preimages. The first checkpoint is not accepted as a reproduced
source build. Its evidence remains unmodified. The next integrated checkpoint
must retain complete source preimages and coherent shared dependencies.

Reproduce the independent static/frozen evidence review:

```sh
python3 tools/linux/loaded_activity_reporting_review.py --output target/linux/loaded-activity-reporting-independent-review.json
```

The optional `--replay-json` and `--retained-executable` arguments validate the
retained-executable replay against its exact executable hash and every original
payload field. They do not rebuild it or execute original game code. Raw static
placement review remains private at
`../c6-ghidra/evidence/linux-loaded-activity-reporting-review/placement.asm`.
