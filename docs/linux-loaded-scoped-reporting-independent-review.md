# Independent scoped reporting review

The `6bdb67d` checkpoint includes the scoped ReportingContext from `6132bef` and
nine isolated source-projection cases. The genuine loaded action still stops at
the missing Manager script backend after four matching native Send scalar rows.
The nine projection cases are separate fixture setups and do not extend that
four-send action trace. Complete state/actions/turns and affected-state/RNG
parity remain unfinished.

The [review tool](../tools/linux/loaded_scoped_reporting_review.py) checks all 167
retained source preimages against the coherent commit, verifies the generated
Cargo manifest/lock and executable identities, replays the retained Rust checker,
and compiles six focused safe-API probes. The
[review report](../evidence/linux/loaded-scoped-reporting-independent-review.json)
records the exact checks and result. The retained executable replay reproduced
all 29 result fields, and all six compile probes behaved as specified.
No original process runs and the unchanged
original method-body census is not repeated. The earlier independent `577b700`
source rebuild is reused for the 163 unchanged preimages; the four changed files
are the shared action module, reporting module and two checker sources. This
review does not relabel the new retained executable replay as a fresh full fixture
source rebuild.

ReportingContext's state and reference fields are private. Its constructor is
restricted to the parent module, `state()` returns an immutable LoadedState,
`reference()` returns a copy, and `actor_mut()` resolves only that fixed addressed
Unit. External safe providers cannot acquire mutable access to another Unit,
Player, Formation, Map or gameplay RNG through this interface. The compile
probes admit ordinary reads followed by actor activity changes, reject mutable
world/player access and reference replacement, and reject a live mutable actor
borrow overlapping an immutable world borrow. The concrete reporter uses only
the immutable state view.

`actor_mut()` exposes the entire Unit, so identity writes are **not statically
forbidden**. A positive compile probe makes this explicit. They remain outside
the supported runtime mutation subset: after callback success, the shared report
boundary rereads the addressed record and compares all fields except activity.
An owner change causes the existing owner validation to reject; an ID or other
record-field change fails the comparison before action continuation. The fixed
context reference cannot redirect mutation to another slot. Native low16 recycled
lookup aliases remain intact. Subsequent runtime queries check current logical
identity against the archive identity and compare initialization dependencies;
they do not silently accept a stale sidecar after identity/dependency changes.

These are source guard conclusions, not newly exercised native identity-mutation
fixtures. Unsupported actor writes and completed prefixes remain present on
errors; there is no rollback or world-index repair claim. The current continuation does not accept
such an unsupported mutation as a successful action. The API does not globally
quarantine the failed state; subsequent diagnostics require consistent owned
state and cannot assume stale placement indices were repaired. Callbacks requiring wider
World/RNG mutation must reject this diagnostic context until an adequate mutation
boundary is implemented.

The real prefix, its three snapshot paths and its exact script failure are
unchanged from the previous review. The empty-AWAKE cleared-report snapshot still
computes direction 3 to (80,57), HasMoved=true and cost 1 within remaining integer
movement 1. It bypasses HoldCycling for that call. Direct native getter comparison,
whole-action effects and gameplay RNG coverage remain separate gaps.

The nine isolated cases use fresh reporting providers and explicit current-Unit
queue/activity setup. Their outputs are stored in a separate array and never
appended to the genuine action trace. All nine agree with the source-derived
host event/scalar rules. Added/Started read subtype and first integer;
Deactivated/Cleared read first and second integer, with absent EventData scalars
zero. Activity AWAKE/HOLD retains operation 0; the isolated OPERATION case with
the admitted current SkipTurn front yields hash 745,019,656 and four parameter
defaults `[-1; 4]`. The actual parameter-free writer leaves the fresh map empty.
These cases now exercise the compiled Started and OPERATION projection branches
as isolated Rust fixtures; their resulting host fields are not captured native
host-publication values and do not prove the remaining five action sends.

Actual callback/output omissions remain under the existing stock headless scope.
No playback consumers or successful script defaults are invented. The retained
result explicitly preserves the original native-report configuration/input
attribution separately from the owned snapshot and marks the comparison scope.

```sh
python3 tools/linux/loaded_scoped_reporting_review.py --replay --capability-probes --output target/linux/loaded-scoped-reporting-independent-review.json
```

The retained candidate and complete source package are under
`target/linux/loaded-activity-reporting-candidates/762d0ed449504fb0b8b1b51c79a8084a/`.
Its executable SHA-256 is
`a7c6f701aa904906a5999b7d77c42bfb75a3fd5581f3b38be3dc64b93f4546db`.
Historical checkpoints and their stated limitations remain unchanged.
