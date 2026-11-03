# Independent ordered reporting checkpoint review

The retained `577b700` checkpoint advances the real loaded SkipTurn prefix through
four synchronous sends and source-derived host publications 35, 70, 71 and 67.
It then rejects the genuine missing Manager script backend during activation.
This is partial source execution over the captured world, not complete gameplay
state/action/turn or affected-state/RNG equivalence.

The follow-up [review tool](../tools/linux/loaded_ordered_reporting_review.py)
verifies all 167 retained source preimages and the retained executable, independently
rebuilds the Rust checker offline with its resolved Cargo lockfile, and compares
the complete checker result payload. The generated Cargo manifest, lockfile,
source manifest, source files, executable and result remain under
`target/linux/loaded-activity-reporting-candidates/b369f6cd8e844bfb9a70cb761cf2686f/`.
All 167 source preimages also match the coherent frozen commit `577b700`.
The original checkpoint executable is
`eb15160822a24373ef2e404af6f409f1d5a1cf8b25b691deb004a73611fbcf74`.
The [review report](../evidence/linux/loaded-ordered-reporting-independent-review.json)
records the independent build's identity and comparison. No original process was
executed. The previous first-checkpoint reconstruction limitation remains
historical and is not retroactively repaired.

Exactly **4 of the 9 captured typed Send rows** match, including event type/subtype,
ComponentID and optional integer arguments. These are native Send scalar rows;
they are not captured native host publication or computed snapshot values.
The source-derived translations preserve the unusual scalar mapping:

| Event | Host event | Operation field | Value field |
| --- | ---: | --- | --- |
| Activity AWAKE | 35 | Constructor default 0 | Generic projection field -1 |
| Operation deactivated | 70 | First integer 1,477,390,184 | Second integer 1 |
| Operations cleared | 71 | Missing first integer becomes 0 | Missing second integer becomes 0 |
| Operation added | 67 | Type subtype 0 | First integer, runtime handler ID 36 |

OperationAdded's first integer must not be substituted for its subtype. The
native translators at `0x320cb30` and `0x320cbb0` read EventData+4 and +0x50;
Deactivated `0x320ccb0` and Cleared `0x320cd30` read +0x50 and +0x54. The review
checks those encoded scalar loads and fingerprints only eight relevant
translator/constructor/facade bodies. It reuses the unchanged earlier 266-body
review instead of repeating that entire census.

This checkpoint now exercises three snapshot readiness calls. The first sees
AWAKE with the old MoveTo node and returns MemberNotReady. After the old node is
removed, the cleared report sees the authoritative empty queue and computes an
adjacent move. Native direction 3 maps (81,58) to (80,57); the source query finds
HasMoved=true and Plot movement cost 1 within the remaining integer movement 1.
This positive branch bypasses both HoldCycling callbacks in the pinned native
CanCyclePast body. The added report then sees the new pending node and again
returns MemberNotReady. This is an actual Rust fixture branch, rather than the
unexercised adjacency implementation in the first checkpoint. Native direct
HasMoved/Plot-cost query parity is still unavailable; the separate historical
native AL=1 prefix is not silently reused as a same-state query oracle.

Existing concrete consumer proofs continue to support omitting reflected Unit
output storage and the stock UnitFlag visual dirty history. The actual captured
prepublish callbacks remain null for the admitted stock sequence. No successful
empty callback table or playback drain is invented. Nonempty/custom property
copying, foreign placement, callback fallbacks and later gameplay consumers keep
their explicit unsupported boundaries. The stock/no-added-consumers scope is an
explicit caller condition.

UnitOperationStarted host68 and the OPERATION activity facade projection are
compiled but **not reached** by this checkpoint. Static review confirms the
facade reads the current front handler's operation hash and invokes the current
parameter writer into a fresh map. The implemented SkipTurn branch requires its
admitted actual handler and parameter-free writer; other front shapes reject.
Constructor operation 0 and four parameter defaults -1 remain distinct. Those
branches have no native host-output comparison or exercised acceptance here.

The concrete reporting provider reads the same authoritative LoadedState through
an immutable borrow and retains copied identities/scalars. A script failure keeps
the completed queue/activity prefix and pending publications intact. The frozen
shared extension API still allowed other providers a wider mutable-world borrow;
its addressed-Unit comparison alone did not police every possible other-world
write. That limitation does not occur in this concrete immutable-query provider.
A later restricted ReportingContext change belongs to a separate checkpoint and
is not attributed to this build.

```sh
python3 tools/linux/loaded_ordered_reporting_review.py --checkpoint evidence/linux/loaded-ordered-reporting-check.json --rebuild --output target/linux/loaded-ordered-reporting-independent-review.json
```

The remaining stop is a real dependency, not an authorization prompt or a reason
to force the remaining five scalar rows. Complete post-load gameplay, native
handler query comparison, full action effects, all gameplay RNG and full turns
remain unimplemented or unvalidated.
