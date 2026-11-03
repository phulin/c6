# Independent full Manager CLI and driver review

The FULL-only wiring at `91b0159`, corrected by `ddb293a`, has no remaining
blocking source defect in this review. This accepts the wiring for integration;
the sole runner must still admit the final compiled preload, observer plan,
source preimages, configuration and runtime identities before original execution.

The independent review found a real cleanup defect in `91b0159`: after a
successful debugger launch, a failed first process-identity read raised before
retaining an identity and skipped disposal. A harmless Python sleeper substituted
for GDB remained running after that refusal; the test cleaned it up. The owner
fixed this in `ddb293a`. The first debugger script block now waits for an atomic
parent authorization and verifies its own PID, starttime, session and executable
before `starti`. An unadmitted direct child is terminated and waited, using a
pidfd where available; no unproven session is enumerated or signalled. This
separates early child cleanup from later identity-checked session disposal.

FULL selection is mutually exclusive with action and passive fixture modes.
Its actor input permits owner/full ID/visibility only, with duplicate keys,
wrong scalar types and expected-answer fields rejected. It requires the loaded
main-thread lifecycle, cleanup, private fresh artifacts and an action trace.
Execution requires the exact expected configuration; build-only returns before
launch, and preparation records the selected paths and static plan. Internal
four-Save diagnostics and outer two-Save diagnostics are independently optional,
all-or-none and distinct from observation paths. Metadata completeness or Save
equality does not become a predicate gate. Existing non-FULL loads retain their
required outer Save paths. New source/profile fields change configuration hashes;
old prepared hashes must not be silently reused.

The linked source list includes the reviewed wrapper, caller, owner guard and
passive metadata helper, with the existing lifecycle and trace implementation.
The Python dependency inventory follows repository imports recursively and
includes the driver and observer. The driver verifies the admitted static plan,
builds a runtime gate from the exact prepared configuration/source map and
installs the observer in fresh `observer/capture/`, while retaining supervisor
artifacts in `observer/`. It does not invoke the predicate separately or retry
it. The already reviewed wrapper/caller owns the one requested predicate;
observer runtime validation remains separate.

The supervisor removes inherited C6/LD variables from its own debugger
environment, passes the selected fixture variables through the command file,
and rejects embedded newlines. Normal status comes from both debugger and
observer terminal results; missing/malformed footer, observer failure, debugger
failure, timeout and live session members cannot become success. Cleanup uses
the retained launch identity and owned child set. Zombies alone do not count as
executing processes. A nonzero inferior result is preserved. The driver does
not claim that disposal proves native cleanup or complete gameplay behavior.

The [independent test](../tools/linux/full_manager_wiring_review.py) passes
**32 cases**: 16 executions of the actual CLI's pure validation prefix, 13 mock
driver cases, and three harmless real Python children executing the exact
generated authorization block. Missing identity and wrong session both leave
the child waited, with no authorization or progress beyond the barrier. The
valid identity passes before progress. No GDB or original game code executes.
CLI tests stop before asset validation/build and use synthetic stream identity;
they are not admission tests for actual game inputs.

```sh
python3 tools/linux/full_manager_wiring_review.py --output /tmp/full-manager-wiring-review.json
```

The [result](../evidence/linux/full-manager-wiring-review.json) pins stable
source hashes. The initial build's immutable source archive at
`target/linux/full-manager-native-1-source-91b0159/` was separately checked:
all 92 complete preimages and archive hash matched, every quoted C dependency
was included, and differences from commit `91b0159` were an empty list.
Its archive SHA-256 is
`c5942f38818bab5721a2633cbb3c43a07bf5bbe69dc2c108520702a0753fd10e`.
That archive predates the driver fix; final preparation must retain refreshed
driver/CLI preimages and identities rather than relabel it as the final build.
This review supplies no native predicate result, callback/RNG completeness or
state/action/turn parity claim.
