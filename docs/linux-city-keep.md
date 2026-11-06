# Keep a captured city

`Session::keep_city` resolves the supported Linux captured-city disposition from
an external policy. The current profile is an ordinary combat transfer between
surviving major players, with the surprise-war occupation callbacks qualified.
Complete Civ VI state, arbitrary dispositions and full turns remain unimplemented.

Read `Session::pending_city_disposition` after completed capture. Construct
`city_disposition_actions::Definitions` from the same rules and state, then call
`keep_city` with the owning player, pending City component, civilization levels,
phase definitions, mandatory current callback services and the notification budget.
The request checks current ownership, legal Keep membership, phase and pause state
without invoking the ordinary-action guard that blocks this pending choice.

The callback services must borrow the ongoing diplomacy, requirement/modifier,
territory, unit, script and loyalty owners. Keep clears the capture/rebellion
markers, updates occupation, executes the ordered occupation callbacks and current
loyalty queries, then refreshes normal blocking state twice and finishes message
processing. It removes the semantic queue entry only after success. Historical
combat records remain unchanged. Unsupported reached branches preserve their
executed prefix and freeze the Session, including after extracting and rewrapping
its parts.

A typed initialized snapshot may import a completed pending disposition through
`stock_notifications::Runtime::initialize_completed_city_disposition`. This derives
the decision from the actual current City, center, capture marker and legal
branches. It does not invent a historical melee record. Choices queued inside an
incomplete live capture remain unavailable until that capture finishes.

The retained native case completes Keep through the persistent Session. City,
player, unit, map, visibility, diplomacy, requirement/modifier, loyalty, history and
represented RNG state match. It adds 75 occupation grievances and activates a
first diplomatic penalty of raw Q8 -4608 (-18 units). The capture choice disappears;
normal blocking changes from Consider Raze to Research, leaving the next player
choice external. Pending-choice persistence, ordinary-action blocking, wrong-owner
and wrong-City rejection are checked separately on the same initialized input.

The supported callback profile excludes active spy-mission aborts, positive unit
expulsion placement, other war types and reactivation of an existing diplomatic
penalty tracker. Dead/free-city previous owners and cultural transfers require
separate transitions. Raze, Return and Liberate require their own native
qualification. Known GameSync, script RNG and native endpoint checks do not
establish a complete gameplay RNG inventory or arbitrary action chaining.

One Cargo release sample measured **2.718 ms** (2,718,128 ns) for the complete
`Session::keep_city` request, reached gameplay callbacks and blocking/message tail.
It excludes import, fixture qualification, service setup, negative-boundary checks
and AFTER comparison. This is one bounded action sample, not full-turn throughput
or a native speedup claim.
