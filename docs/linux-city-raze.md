# Raze a captured city

`Session::raze_city` lets an external policy resolve a supported Linux captured-city
decision. Raze destroys the City immediately. The qualified profile is an ordinary
noncapital combat capture between surviving major players during a surprise war.
Complete Civ VI state, arbitrary actions and full turns remain unimplemented.

Read `Session::pending_city_disposition`, construct
`city_disposition_actions::Definitions` from the current rules and state, and call
`raze_city` with the owner, pending City component, civilization levels, phase
definitions, mandatory current services and notification budget. The request checks
current ownership, native razing legality, the completed pending choice, strategic
phase permission and pause state. It leaves policy selection external.

The services borrow the ongoing diplomacy, requirement/modifier, City destruction,
territory and sight owners. Raze clears the capture/rebellion markers, publishes
command start, increments the razer's counter against the previous owner, dispatches
the ordered
CityRazed consumers and executes non-transfer destruction. This shares capture's
City/district lifetime, modifier release and territory cleanup. Spy displacement,
park removal and post-destruction naval relocation occur at their source positions;
unsupported positive branches stop explicitly. Normal blocking updates and message
processing finish before the pending choice is consumed. A mutating failure retains
the executed prefix, freezes the Session and preserves its pending decision.

The native initialized case completes Raze through this persistent Session without
inventing historical combat. The City and its center disappear; surviving City and
Unit identities, ownership, map, visibility, diplomacy, requirements/modifiers,
CityBuild lifetime, history and represented RNG state match. It adds 225 grievances
and a first diplomatic tracker of raw Q8 -5120 (-20 units). Native immediate
concrete removal leaves that tracker inactive with its 20-turn cooldown and value
retained. The triggered razing requirement releases its final reference and
unsubscribes during its own dispatch. Pending-choice persistence, ordinary-action
blocking, wrong-owner and wrong-City rejection also match the expected boundary.

This profile excludes existing diplomatic tracker reactivation, other war types,
dead/free-city previous owners, capitals, matching spies, positive park removal,
naval relocation and unsupported teardown consumers. Return and Liberate remain
unimplemented. Known GameSync/script RNG and native endpoint comparisons do not
establish a complete gameplay RNG inventory or arbitrary action chaining.

One Cargo release sample measured **2.770 ms** (2,769,532 ns) for the complete
`Session::raze_city` request, reached callbacks, destruction and blocking/message
continuation. It excludes import, fixture qualification, service setup, negative
checks and AFTER comparison. This is one bounded action sample, not full-turn
throughput or a native speedup claim.
