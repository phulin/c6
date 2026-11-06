# Ordinary Granary selection and growth

The persistent Rust `Session` supports selecting an ordinary Granary with its
technology prerequisite fulfilled, and completing a queued Granary through
bounded normal Linux turns. This is representative early-game coverage;
complete Civ VI state, arbitrary actions and full turns remain unimplemented.

`Session::choose_building` checks the current owner's acquired technology flag
using the same canonical lookup as unit production. The choice runs ordinary
queue insertion, construction-plot setup and current report queries. Normal city
turns borrow the existing building-completion registrations and share the
Session's modifier, requirement, Script, worker and religion owners.

A separate original selection comparison starts with an empty city queue after
full Pottery acquisition. `CanConstruct` and `Session::choose_building` accept the
Granary; ordinary insertion emits ConstructionStarted, ProductionChanged and
QueueEntryChanged in order. The queue becomes one entry with no seeded progress.
All compared typed components, player fields, rosters, core processors and tracked
RNG match the original selection action.

Production precedes growth in the original city activation order. A completed
Granary therefore contributes its rules-defined Food and Housing to that same
activation's growth queries. Population changes run the existing citizen,
religion and synchronous reporting callbacks; subsequent turns query the current
building and worker state again.

The completion comparison starts with an originally legal Granary queue one Q8
production unit below its actual cost of 65 and stored food one Q8 unit below its
current growth threshold. Original APIs prepare this fixture. Ordinary turn
completion finishes the Granary, grows population from one to two, and retains
food 767 Q8 and production overflow 1023 Q8. Housing rises from 6 to 8. After an
external Monument choice, the same Session executes the next normal turn:
current Food yield is 1792 Q8, food surplus is 768 Q8, and stored food becomes
1535 Q8. These amounts are comparison observations, not injected mechanic results.

The entire typed City matches at the completion boundary, including buildings,
queue, citizens, growth and religion. Both live Food/Housing checkpoints and the
final global building counts match. Compared final gameplay components, rosters,
core processors, represented requirements and tracked global RNG/libc state also
match the pinned [Linux target](targets.md). Raw external-policy/control and
wire-annotation differences remain recorded; complete RNG and runtime coverage
remain unestablished.

The supported scope has ordinary stock subscribers, empty current gossip
visibility history and the source-qualified early-city achievement path. A
missing technology prerequisite remains explicitly unsupported: the native
wrapper's result differs from its inner requirement result in the captured
unacquired-Pottery diagnostic. Civic, placement, wonder and other unqualified
prerequisite branches remain explicit errors. Unsupported transitions retain
any already executed mutation prefix.
