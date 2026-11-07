# Ordinary rounds after trade-route war interruption

One persistent Rust Session can start the supported foreign route, declare
surprise war, and continue through three ordinary wartime rounds to the next
external player 0 decision. The Session, mutable callback owners and RNG remain
retained throughout; no native AFTER state initializes the candidate. Complete
Civ VI gameplay, arbitrary actions and arbitrary full turns remain incomplete.

This stock Linux XP2 case uses the two founded original capitals from the
[war-interruption profile](linux-trade-war-interruption.md). Human and NPC
choices remain external: six recorded human handoffs and twelve NPC unit-policy
DEFER responses are consumed alongside explicit research, production and unit
orders. Ordinary upkeep includes barbarian unit creation. These rounds contain
no combat orders.

The continuation preserves positive influence-path jobs, war knowledge,
relationship/opinion updates, capacity tracking and callback ordering. The
informational available-capacity notice is omitted at the qualified headless
output boundary while its notified-capacity write executes. A rejected Backstab
emergency query still creates and releases its Emergency context, requirement
and set. Seven reached combat previews create and forget proposals in source
order using the same requirement counter; their pure policy scores are omitted.
The original preview damage branch consumes no damage RNG.

At turn 3, owner 0 is ready for an external decision. The Trader is awake at its
origin with an empty queue and two moves. Edge 0 remains inactive with the same
path and started/removal counts 1/1; the next real route ID remains 1. Neither
trading post is constructed. The native proposal counter is 7839, the shared
requirement counter is 18993, and no proposed combat remains active.

The optimized comparison uses public `970e83fc` and private `495cc93a` on the
[pinned Linux target](targets.md), configuration
`a07f33546b3710f2b3946ceaf1074c77f94b9acaee2b8e72880e9917aa485010`.
Private `c6-re/evidence/linux/trade-war-rounds-session-21-release.json` records the
exact source/rules/input identities, executable, command, build profile and
result. All four trajectory checkpoints match represented gameplay and
GameSync. Terminal typed gameplay, core processors, known gameplay RNG/libc,
requirements, callback IDs, modifiers and opinion state match. The existing
route-interruption and no-route declaration regressions also pass with that
same optimized build.

Raw full-action equality remains false. Source-qualified native policy output
and intermediate unsaved District observations remain visible in the receipts;
this does not establish a complete RNG inventory. Visible or remembered hostile
Unit forecast targets, other planner shapes, positive emergency eligibility,
active military operations and executable behavior trees remain explicit
unsupported branches. The interruption profile's deal, foreign-governor,
shared-sight and expulsion restrictions also remain.

In one optimized replay, the three-round Session requests took 109.8 ms. This
measures advance/resume/research and recorded handoff commands. It excludes
fixture import, provider setup, initial trade/war commands, checkpoint
comparison, external policy selection and JSON serialization.
