# Loaded Unit activity reporting

`loaded_actions::reporting` resolves concrete synchronous Unit lifecycle
SkipTurn reporting paths over the authoritative `LoadedState`. It is a bounded
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
typed publication at the original translation point; it does not invent
its later subscribers or move synchronous work to action completion.
`drain_playback` explicitly rejects unavailable queued consumers.

The reflected Unit snapshot/CacheUpdated275 output is omitted under its concrete
consumer proof. Stock UnitFlag dirty vectors are separately omitted under the
[stock gameplay scope](linux-reporting-gameplay-scope.md). Neither omission
creates an empty registration table. `Scope::StockHeadlessNoAddedConsumers` is a
caller admission condition for the active profile, not evidence automatically
obtained from Lua source hashes. Nonempty/custom Unit property copying, non-SkipTurn
facade projections and subsequent event types remain explicit dependencies.

The implementation receives an immutable authoritative world view through
`Reports::send_loaded` and `ReportingContext::state`, after the setter has released
its short actor borrow and before the next action operation. The context permits
mutation only through its checked addressed-actor accessor; it does not expose
other mutable World components or RNG. No World clone, actor copyback or second
mutable actor alias is used. The old Unit-only entry rejects this provider because it
cannot resolve the required owned-world inputs. Reports returning errors leave
the already performed action prefix intact.

The [focused source audit](../tools/linux/loaded_activity_reporting_audit.py)
checks 273 inherited and additional method bodies against the pinned Linux XP2
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

Historical checkpoint `9491031` checks the first report while owner 0 /
Unit 15,794,187 still retains its
single old MoveTo queue node. It has AWAKE activity and remaining moves 256 Q8.
The original member-readiness pending-operation guard therefore returns false,
so this first snapshot never reaches CanCyclePast or HoldCycling. One event35
publication is prepared, and the real action prefix advances to
UnitOperationDeactivated `a60328c5`, which that checkpoint did not yet handle. The loaded check records this branch instead of claiming it exercised
the implemented adjacency evaluator. Shape/identity rejection and unavailable
playback drain preserve both the pending publication and authoritative state.

The source extension additionally implements the observed deactivated, cleared,
added and started translators (host events 70, 71, 67 and 68). It retains their
literal scalar copies: Added/Started use Type.subtype and first; Deactivated/
Cleared use first and second. EventData1 initializes absent scalar fields to
zero. Added's captured subtype 0 / first 36 stays exactly that; the adapter does
not replace it with a guessed operation hash. Cleared, Added and Started perform
their actual synchronous Unit snapshot effect check before host translation;
Deactivated has no observed cache callback.

For an OPERATION activity, the provider resolves the actual current queue at
that boundary and requires an admitted current SkipTurn handler and its concrete
empty parameter writer. It returns the original SkipTurn hash; a genuinely empty
queue returns zero. Fresh host activity output initializes operation to zero and
all four optional parameter values to -1. Unknown front handlers or parameter
writers reject instead of synthesizing a facade. Host events retain their
original common order. These are source-derived fields; the original nine typed
Send scalar rows did not capture computed host publication or snapshot output.

The subsequent checker retains full immutable source preimages, a source
manifest, generated Cargo manifest/lock and candidate binary under
`target/linux/loaded-activity-reporting-candidates/`. This supersedes the first
checker's reconstruction limit: its temporary full source directory was deleted,
and its recorded shared `loaded_actions.rs` hash differs from the checkpoint's
Git blob. That historical report remains unchanged and is not a reproducible
source rebuild claim. New whole-entrypoint comparison covers only the prefix
actually executed before the genuine script/report failure. Additional isolated
report-shape fixtures remain separate from that prefix. Neither comparison
establishes native affected-state or RNG equivalence.

The retained `577b700` candidate completed the first four actual report calls:
host publications 35, 70, 71 and 67, with all four original Send scalar rows equal.
The empty-queue AWAKE snapshot now exercises the real adjacency evaluator: native
direction 3 selects (80,57), HasMoved is true, and Plot cost 1 fits remaining
integer movement 1. The pending-node snapshots before and after that boundary
return through the member-readiness guard. The next genuine stop is the Manager
activation ScriptSystem provider; no later action report is manufactured.
Its complete retained source and candidate/result are recorded by
[the ordered check](../evidence/linux/loaded-ordered-reporting-check.json).

The scoped follow-up also tests nine isolated source projections, one per
captured Unit Send row. Each case uses a fresh reporting provider and an
explicit actor queue/activity fixture based on the actual old MoveTo queue or
the SkipTurn queue created before the scripting failure. These test setups
occur after the genuine prefix has been recorded. Their rows remain a separate
array and never extend the action trace. They test the previously unexercised
SkipTurn OPERATION facade and empty-map parameter defaults, plus Started/HOLD
and later deactivation/cleared shapes. They are source projection tests, not
nine successful action calls or a native host-output comparison.

The retained scoped candidate at `6bdb67d` passes that follow-up: the genuine
four-report prefix remains unchanged, and all nine independent projection cases
pass, including the SkipTurn facade hash and four -1 parameter defaults. Its
[result](../evidence/linux/loaded-ordered-reporting-scoped-check.json) identifies
the separate original report configuration `d68604a5...` and the common exact
80,657,462-byte input snapshot `ff31f4c5...`. Equal scalar rows do not promote
the distinct observer profiles into one live initialization authority. The
[source audit](../evidence/linux/loaded-ordered-reporting-scoped-source.json)
retains the 273 method-body checks. The full source preimages, generated manifests,
candidate and result remain at the paths recorded in that result.

```sh
python3 tools/linux/loaded_activity_reporting_audit.py --output target/linux/loaded-activity-reporting-source.json
python3 tools/linux/loaded_activity_reporting_check.py --output target/linux/loaded-activity-reporting-check.json
cargo clippy -p c6-data --lib --offline -- -D warnings
```

The original raw **89,198-byte Save→Load difference** remains unclassified and
unmasked. Save production is not a gameplay acceptance requirement.
