# Policies through ordinary upkeep

The persistent Rust `Session` supports explicit bounded policy selection followed
by ordinary Linux turn progression with the policy's city yield effect retained.
This is representative early-game coverage, not complete Civ VI policy or turn
support.

External policy requests use `Session::request_policies`; ordinary turn services
then reuse the same canonical city, research, callback, requirement and effect
owners. Culture upkeep checks active policy flags and replacement-policy unlocks
in original slot/rule order. A retained policy's production bonus feeds the next
ordinary city production calculation. The simulator never chooses a policy or
ends a player's turn for them.

The checked founded-city scenario explicitly selects Discipline and Urban
Planning under Chiefdom after three research rounds, then advances one normal
round in the same Session. Urban Planning raises city production from 1024 to
1280 Q8; Monument progress advances from 3072 to 4352 Q8 on the following upkeep.
The entire City build queue and Culture match the pinned [Linux target](targets.md),
along with compared typed gameplay components, core processors, represented
requirements, canonical Game sync RNG and matched libc state. The next genuine
player decision remains external.

A second checked scenario completes Enlightenment through ordinary culture
progression while Urban Planning is active. Its Rationalism replacement unlocks,
Urban Planning is removed, and its production modifier is detached: city
production returns from 1280 to 1024 Q8, with Monument progress at 4096 Q8.
Culture, city build queue/yields, newly attached research modifiers, represented
requirements and typed gameplay components match the Linux reference. Rationalism
remains inactive; the pending policy-slot decision exposes its legal slot to the
external policy. The simulator does not select the replacement.

GreatPeople-dependent obsolescence and XP1 age-restricted upkeep remain explicit
unsupported continuations. Other unknown
branches retain completed mutations and return partial-transition errors.
Complete modifier/runtime coverage and the complete RNG inventory remain
unestablished; raw native AI policy snapshots differ. This bounded result does
not establish arbitrary actions or full-game turn equivalence.
