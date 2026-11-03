# Loaded Unit activity reporting

`loaded_actions::reporting` resolves a concrete part of the first synchronous
SkipTurn reporting path over the authoritative `LoadedState`. It is a bounded
source implementation, not complete action acceptance. Complete gameplay state,
actions, turns, affected-state comparison and gameplay RNG remain unfinished.

The first activity send (`e23b112d`) reaches the actually observed Unit cache
receiver `27ff430`, then `PublishUpdate 27ff860`, `Add 27ff2d0` and snapshot Fill
`2805100`, before host translation `320b670`. Existing [Unit output consumer
evidence](linux-unit-event-consumers.md) proves this particular reflected Unit
stream is output for the external policy/UI. Its snapshot values, event keys,
allocator peaks and desktop heap layout may be omitted. Existing [snapshot
effect evidence](linux-unit-event-effects.md) reviews the concrete getters and
retains a meaningful exception: readiness can reach command/operation
HoldCycling callbacks. Omitting output does not justify omitting those effects.

The new provider checks that exception synchronously at each admitted report.
It resolves the actual current Unit and initialized Formation binding, performs
the original member-readiness guards and scans other Formation members in their
stored order. If CanCyclePast reaches HasAdjacentMove, it scans the six neighbors
in original direction order, with the native wrapping coordinate arithmetic.
Placement runs before HasMoved. The latter uses the existing [owned-state
movement provider](linux-unit-movement-state.md), including mutable diplomacy
and Game relations. A moved Unit then compares the actual adjacent Plot cost
against its remaining Q8 moves converted to an integer. No captured AL result,
Formation `has_moved` flag or expected readiness answer enters this computation.

The current placement subset reads actual Plot impassability, initialized Unit
invalid-terrain flags, native-order terrain Water definitions, runtime domain,
Unit spy properties and actual owning player. The supported positive branch
requires no foreign-border query, no embarkation/city-domain fallback, no spy
district branch, empty Unit candidate cells and no barbarian tribe continuation.
Earlier unsupported neighbors stop the original-order scan; a later convenient
neighbor cannot be substituted. Terrain rejection may omit the later reviewed
pure queries because the snapshot output itself is excluded, but never skips a
possible HoldCycling invocation. If all neighbors fail, the provider explicitly
requires command HoldCycling followed conditionally by operation HoldCycling.
These callbacks are not silently classified as planning or UI.

At host translation, AWAKE and other non-OPERATION activities bypass both
operation facade getters. Type.subtype is zero; EventData's separate Unit type
`6447c33d` comes from the identity parameter. The actual three activity35 and
seven CacheUpdated275 prepublish callbacks are already proven null by the
[closed reporting capture](linux-skip-turn-reporting-closure.md). Their absence
is scoped to that admitted stock reporting path. The provider retains an ordered
typed event35 publication at the original translation point; it does not invent
its later subscribers or move synchronous work to action completion.
`drain_playback` explicitly rejects unavailable queued consumers.

The reflected Unit snapshot/CacheUpdated275 output is omitted under its concrete
consumer proof. Stock UnitFlag dirty vectors are separately omitted under the
[stock gameplay scope](linux-reporting-gameplay-scope.md). Neither omission
creates an empty registration table. `Scope::StockHeadlessNoAddedConsumers` is a
caller admission condition for the active profile, not evidence automatically
obtained from Lua source hashes. Nonempty/custom Unit property copying, operation
facade projection and subsequent event types remain explicit dependencies.

The implementation receives the actual authoritative world through
`Reports::send_loaded`, after the setter has released its short actor borrow and
before the next action operation. No World clone, actor copyback or second mutable
actor alias is used. The old Unit-only entry rejects this provider because it
cannot resolve the required owned-world inputs. Reports returning errors leave
the already performed action prefix intact.

The [focused source audit](../tools/linux/loaded_activity_reporting_audit.py)
checks 266 inherited and additional method bodies against the pinned Linux XP2
SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`
and host `106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
It reuses the existing callback/consumer evidence rather than reopening absent
prepublish callbacks or DirtyComponents registrations. No original execution,
new observation profile, save writer or Windows work is performed.

The [loaded fixture checker](../tools/linux/loaded_activity_reporting_check.py)
builds an immutable source snapshot, reads the existing closed full-world
fixture, initializes its 1,262 Unit queues using all 63 actual handler bindings,
and supplies the real reporting provider to the actual loaded SkipTurn prefix.
Its missing script provider still rejects genuine script calls; it never returns
successful callback defaults. Its result records the exact next unsupported
consumer and ordered activity publications. Source replay does not establish a
new native differential action comparison.

The actual first report occurs while owner 0 / Unit 15,794,187 still retains its
single old MoveTo queue node. It has AWAKE activity and remaining moves 256 Q8.
The original member-readiness pending-operation guard therefore returns false,
so this first snapshot never reaches CanCyclePast or HoldCycling. One event35
publication is prepared, and the real action prefix advances to
UnitOperationDeactivated `a60328c5`, which this provider explicitly does not yet
handle. The loaded check records this branch instead of claiming it exercised
the implemented adjacency evaluator. Shape/identity rejection and unavailable
playback drain preserve both the pending publication and authoritative state.

```sh
python3 tools/linux/loaded_activity_reporting_audit.py --output target/linux/loaded-activity-reporting-source.json
python3 tools/linux/loaded_activity_reporting_check.py --output target/linux/loaded-activity-reporting-check.json
cargo clippy -p c6-data --lib --offline -- -D warnings
```

The original raw **89,198-byte Save→Load difference** remains unclassified and
unmasked. Save production is not a gameplay acceptance requirement.
