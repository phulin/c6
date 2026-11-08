# Retained barbarian Scout kill and ordinary round

The bounded Linux profile completes a second legal Scout attack, the original
barbarian Scout's death and retirement, and an ordinary round in one persistent
Rust Session. Complete gameplay state, arbitrary actions and arbitrary full
turns remain unimplemented.

The retained prefix includes wonder discovery, Alpine, the actual tribal-village
Bronze Working boost, flood expiry, the original Settler's first city, border
growth, Pottery, Monument, Code of Laws, external Discipline and Urban Planning,
and the prior surviving melee. All player choices remain external. The second
attack uses the actual surviving participants and current movement, health,
strength, sight and RNG; it does not reset or import an endpoint.

At turn 24, owner0 Scout196610 attacks owner63 Scout131073 at (16,18). The original
CanStart accepts modifier16 MoveTo. Attacker damage changes 23→44; defender damage
78→100. The victim remains delayed-dead and roster-owned immediately after combat.
Ordinary owner63 retirement removes it; the following round reaches owner0
DecisionReady at turn 25. The surviving Scout occupies (16,18).

The supported factory now retains immutable owned-modifier destruction handles
on the existing effect runtime. Death still reads current mutable modifier,
requirement, callback and sight owners, deactivates the Scout's SeeHidden effect,
and performs ordinary cleanup. New barbarian tribes also retain their actual
initialized empty dynamic-property domain. Unsupported effect continuations
continue to return explicit errors.

Both immediate attacks, the pre-kill round boundary and final typed gameplay
match their native observations. The complete exported Barbarian Manager matches
immediately after combat and after retirement: tribe0 loses its Scout, boldness
becomes 41, and the following round yields boldness43/death-age1. Player.Stats
barbarians killed changes 0→1. Known final GameSync is calls791,
seed3859350424, reset0; represented requirements, processors and matched libc
state agree. Raw transport/planning differences remain recorded; complete
runtime state and RNG inventory are not claimed.

Comparison production pins are public `437aacde` / private `e955bb79`. The original
producer uses public `e2bb54d4` / private `23f6d274`, target SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`,
configuration `73efeb1d0f3970ba267796a63d5bbb3107869305ac00315fc77c6395c95dc16b`,
and gameplay rules `c880adaedcfd6da7808302b3b76bb9cad7d3188b35ae78e780cff7b56e04477e`.
The canonical input manifest is
`3894a824fbac3e26cbf2401c1b29779c0789f2faa9f62a4b20dc3d855d87bb3b`.
Private receipts retain exact commands, build profiles, executable/input hashes,
configuration and canonical captures. Production builds remain standalone Rust.

Optimized new24-round, prior23-round and major-lethal regressions pass. The
second attack request measured 2.27 ms, including synchronous gameplay callbacks
and mandatory aftermath, excluding provider construction, input reading,
debug snapshots and JSON. Aggregate round timing includes external actions and
comparisons and is not a pure upkeep measurement. Existing focused lifetime/camp
checks, 1843 workspace tests, all-target Clippy and an isolated public-only build
pass.

The initial native failure remains preserved: its policy shim attempted SkipTurn
on the already dead/delayed victim. The corrected shim records that nondecision
and leaves retirement to the engine. The initial Rust cleanup failure and tribe
property-domain mismatch are likewise retained alongside their fixes.

At the final native boundary the city has population2, 43 production stored in
its queued Granary, Mining research and Craftsmanship civic. A useful next
external action is switching production away and back while preserving stored
progress; legality of any new Builder choice has not been queried.
