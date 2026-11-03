# Independent full Manager wrapper integration review

The wrapper at `13fe520`, with producer audit `de502ec`, has no blocking
source defect in the reviewed caller integration. This review permits source
integration to proceed; it does not admit an original-code run or establish
predicate/gameplay parity. Loader/CLI selection is a separate checkpoint.

The wrapper obtains identity from the already admitted idle scope, validates
the externally selected owner/full ID and paths, and builds a raw actor
address expectation with safe reads. Player mapping is a validity check,
not the pointer-table index. Low16 selects the recycled slot, but the active
list membership, full generation ID, owner and Unit vptr must also match.
This expectation cannot authorize the native call: the real fixture acquires
the genuine facade/lock and resolves the current Manager and actor again
through original getters/Find after both optional pre-call Saves. A replaced
Manager is used; a missing/replaced actor or changed identity is refused.
Neither wrapper nor fixture requires an empty actor queue to ask this predicate.
The request is a full Manager query for SkipTurn, not Execute/Add or a proof of
an external-policy decision boundary.

The original Operation construction, one full Manager call, destruction and
facade release remain in the reviewed fixture. The wrapper forwards all owner
callbacks. Its observations check owner scope independently; Unit and Operation
are address tokens only. After the predicate, the Unit may be gone. The wrapper
does not dereference it, and the fixture clears it after RETURN. The known RNG
reads retain actual bytes at each boundary without equality assertions.
Unreadable required identities/RNG extents remain structural refusal paths,
consistent with the fixture's existing safe-copy contract under retained
Game/module ownership. Optional metadata decoding is not such an admission
requirement.

Metadata capture errors are recorded separately and `observe` returns zero,
so they do not trigger the fixture's observation-status veto. Controls-output
errors are likewise retained without suppressing or repeating the predicate;
a normal return reports output failure `247`. Save diagnostics retain their
statuses and effects. Genuine ownership failure remains fatal before another
original call; the wrapper does not guess that cleanup is safe or restore
mutated state. Fixture errors propagate through normal wrapper cleanup, which
closes controls, releases owned path strings and clears its observation scope.

The independent harness connects the **actual wrapper to actual fixture
`run_bound`**, replacing original game methods, identity/owner services and
metadata capture with mocks. It supplements the producer's wrapper-only audit
with nine integrated cases: false without Saves; preSave payload changes and
Save error; preSave actor deletion or ID change; preSave Manager replacement;
all metadata errors with actor removal during the predicate; controls I/O
failure with actor removal; and disagreements between raw expectation and
the later Find/ID result. On normal returns it asserts one predicate, one
Operation ctor/dtor, one facade acquire/release and four metadata captures.
It protects the actor page after the query and checks retained callback RNG
bytes. Four invalid-actor cases reach the precise fixture refusal.

The shared caller mock uses a separate ID storage location to simulate a
changed getter answer; it is not evidence that native GetID reads that location.
The pinned native `Unit::IInstance::GetID` at `0x2021490` reads `+0xb8`, matching
the wrapper. A narrow read shim supplies the raw Game roster pointer because
the older independent mock Game storage is shorter than `+0x248`. Neither
shim replaces the production wrapper or `run_bound` control flow.

```sh
python3 tools/linux/full_manager_wrapper_review.py --output /tmp/full-manager-wrapper-review.json
```

The [recorded result](../evidence/linux/full-manager-wrapper-review.json)
pins stable reviewed sources and the mock executable. Strict C, GCC analyzer
and UBSan without recovery pass. Actual public original-binding resolution,
real facade acquisition/lock, hardware observer/VM ancestry, mapped target and
configuration admission, full callbacks and complete affected state/RNG remain
separate requirements. No original game code was executed.
