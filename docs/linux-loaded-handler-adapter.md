# Actual handler bindings and owned queue IDs

`loaded_actions::handlers` consumes the independently closed Linux
[current-handler field capture](linux-current-handler-closed-review.md). It
constructs `ObservedRegistry` with actual live slot/code observations and **zero
observed hash-lookup answers**. The existing diagnostic SkipTurn algorithm can
use this admitted source through `skip_turn_state_phase_observed` or
`diagnose_skip_turn_observed`. These share the original implementation; no second
action algorithm or successful callback default is added. Complete state,
actions, gameplay RNG and turns remain unimplemented.

The adapter pins the closed runtime, preparation, configuration, both identical
raw packets and the independent review. It checks the complete original Linux
module SHA-256 and compares the captured live code bytes with executable ELF
segments. Each 16-byte handler object supplies its actual ID at +8, hash at +12
and vptr. Actual method ownership, pointer/base/RVA correspondence, mapped-slot
flags and full three-byte supported interruption leaf are checked before creating
`InterruptionTarget::LiveSlotObservation`. A static image prediction cannot enter
this path. The four observed Manager method bodies are checked too; observing
their addresses and bytes does not establish their return values.

The capture's identity stays unchanged:

| Fact | Actual source |
| --- | --- |
| Observer configuration | `321c39581fd268b9e75d2efaa2a672e8662a3468573bc1cad8bfbec8e15e7045` |
| Raw facts, both passes | `77c31dd6dda9736588ee71c06637eb92ec002b0f7acacd780b30739b9ebef942`, 1,197,257 bytes each |
| Closed runtime | `ce40c14a3eb5882ef04d2524e2231d33b77737a1fc8c542a8ede1cf921184553` |
| Independently reviewed source | `101505b`, review artifact SHA-256 `e0243a249b2971d5a726dfa5f64702a9fd6614eb4527038fd0300367f20d1b15` |
| Associated saved snapshot | `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`, 80,657,462 bytes |

No identity is rewritten to the different decoder/observer profile. Admission
requires the full associated snapshot, pristine loaded state, its separate
observed-snapshot witness and actual resource equivalence through `Bound`.
The original decoder framing role is retained. Matching saved bytes and known
queue fields do not prove equality of unsaved state or native Read effects.

## Source-derived registration and queue initialization

Native [Manager registration](linux-operation-registry.md) obtains a handler's
hash, resolves its actual UnitOperation definition and assigns that definition's
collection index as runtime ID. The adapter reconstructs this registration map
using the genuine 63-definition collection and checks every entry against the
63 actual observed handler objects. This is explicitly source-derived mapping,
not an `ObservedHashResolution` row or an inferred native lookup return. Unknown
or inconsistent registration inputs return errors.

For each of the complete 1,262 ordered Units, the adapter creates an owned initial
runtime-queue diagnostic containing the remapped numeric ID alongside the typed
wire node. It compares every actual queue count and known node field: numeric
operation ID, directive, flags, push turn and previous activity/automation.
All **308 nonempty queues** match; **954** are empty. Membership and ordering are
checked without a placement/player filter. Full raw headers, capacity blocks and
unknown bytes remain retained in the original packet. Shared-parameter pointees
were not captured; the decoded wire payload remains preserved, without claiming
live pointee comparison or classifying unused bytes.

The unique actual SkipTurn object is slot 36/hash `0x2c681908`. Selecting that
object is direct observed-object selection. Source-reconstructed IDs can be used
for newly inserted/current typed nodes without pretending a reverse-map function
ran. Subsequent bounded execution validates the same destination, observed
registry and definition correspondence. The immutable initial-queue diagnostics
remain a baseline; they are not exposed as mutable current queue state after an
action. Current queue reads use the actual loaded records and admitted mapping.

The captured current numeric fronts select 185 MakeTradeRoute leaves and 123
other leaves. The exact three-byte functions imply false and true, respectively.
Those are source-decoded leaf results. The capture did not observe native
GetHandler or CanBeInterrupted return values, and the adapter does not label
these results as a native predicate comparison.

## Concrete fixture and remaining effects

```sh
python3 tools/linux/loaded_handler_adapter_check.py --output evidence/linux/loaded-handler-adapter-check.json
cargo test -p c6-data actual_live_slots_have_no_reverse_lookup_answers -- --ignored
```

The fixture uses the strict attributed import, all 43 Players, all 1,262 Unit and
1,250 Formation initializations, real rules/definition providers and separately
admitted observed snapshot controls/war facts. It compares the complete ordered
runtime queue projection with the raw capture. It then selects a supported actor
through actual typed fields and enters the observed-handler action path.

The genuine reporting backend remains unavailable in this fixture. It returns
an explicit error at the first required report, after the actual loaded activity
assignment. The recorded prefix is Execute lookup → capability preflight → Clear
→ activity-change report. The fixture uses no successful reporting collector,
null-script assumption or no-op aftermath. Its result is a useful partial action
trace, not a successful SkipTurn or complete native affected-state/RNG comparison.
The actor selected in this fixed snapshot is owner 0/ID 15794187; selection does
not reuse that ID as an input constant.

Actual interception/VM/args/locks, required reporting consumers, diplomacy and
notification continuation still need faithful providers and comparison. Only the
interruption slots were newly observed here; other requested-handler dispatch
and gameplay-provider boundaries retain their existing evidence/limits.
The raw **89,198-byte** Save→Load difference stays unclassified and unmasked.
The saved snapshot is an input-attribution diagnostic; native save production
and binary-identical rewriting are not action acceptance gates.

The retained-source fixture records complete source preimages, the generated
Cargo manifest/lock, and the tested executable under
`target/linux/loaded-handler-adapter-candidates/<id>/`. Its public result pins
those artifacts and every source hash. Source snapshotting checks for concurrent
changes before admission; later workspace edits do not alter the tested sources.
The frozen candidate `8fe473590e416276c08ad7fb1dc9c8f6a16dfa666b7a8a2857e02383569ce54c`
checks all 1,262 queues and stops at the first unavailable synchronous report.
The actor initially has timer 0, activity OPERATION (`0x490cc8e1`), remaining
movement 256 Q8, and a MoveTo node pushed on turn 591 with previous activity
AWAKE. These are decoded snapshot fields, not newly observed native query returns.

Its single attempted prefix took 34,999 ns. This excludes decoding, rules,
module/resource hashing and initial all-roster comparisons, and ends before the
first report backend succeeds. It is not full-action throughput. The hot path
currently checks 63 immutable definition/slot pairs once per attempt; it does not
rehash modules or compare all Unit queues on each query. This diagnostic timing
does not establish allocation behavior or native performance parity.
