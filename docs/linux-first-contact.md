# Major-civilization first contact

Ordinary `Session::move_to` sight can discover an unmet major civilization and
yield a first-meeting response on the same persistent Session. The supported
Linux case has a local active human major and an AI major, both cityless, in
single-player mode. Complete diplomacy and arbitrary first-contact states remain
unimplemented.

Initialize the actual current relationship matrices, diplomacy manager inventories,
player controls, capabilities and civilization levels. Bind
[`stock_notifications::first_contact::Inputs`](../crates/c6-data/src/loaded_actions/stock_first_contact.rs)
through `Runtime::bind_first_contact`. Its mandatory `Effects` provider dispatches
the reached reports and meeting subscribers. The provider must use the same live
research, history, requirement, modifier and visibility owners as movement.
Initialize the manager's actual pending-action inventory, including a known empty
inventory when appropriate.

Also bind the current
[unit-view definitions and subscribers](../crates/c6-data/src/loaded_actions/stock_first_contact_unit_view.rs).
Encountered unit types update both the retained player discovery array and its
current service view before dispatching encounter callbacks. Missing state or
unknown reached subscribers return explicit errors.

## External responses

Inspect `Session::pending_first_meetings()` after movement. Each pending decision
identifies its player, session, statement and legal `Positive`/`Exit` choices.
Further ordinary actions are gated until the meeting is resolved.

Use `Session::respond_first_meeting(issuer, choice)` for the initial response.
`Positive` produces an information-exchange statement and another external
response choice. `Exit` closes the conversation, retains its history and clears
the active session.

The final information-exchange `Positive` requires
[`Session::respond_first_meeting_with_update`](../crates/c6-data/src/loaded_actions/session/first_contact.rs).
Supply the current deactivation inputs, turn services and budget. Delivery applies
the result action, including both `had_friendly_meeting` flags, then executes its
mandatory single `Game.Update` using the same retained owners before closing.
These flags record the completed greeting; declared friendship is a separate
relationship.

If that update reaches a genuine player choice, the Session retains its result
continuation. Inspect `pending_choice()` and `pending_first_meeting_update()`.
Matching research choices use the existing explicit technology/civic request
methods; call `resume_first_meeting_update` after resolving the choice. Other
unimplemented decision responses remain explicitly pending. No player policy is
selected automatically. A reached unsupported effect can retain a partial prefix
and freeze the Session; inspect `failure()` before issuing another request.

## Compared boundary and limits

The representative Linux sequence starts with bilateral unmet relations. A legal
Scout move reveals the foreign Warrior and causes first contact. The comparison
covers contact and meeting turns, unit discovery, reached boosts and history,
era score, diplomacy knowledge and active fields, shared requirements and
modifier lifetimes, opinion trackers, conversation identities and reached RNG.

Two complete response sequences are checked against the original Linux target:

- Two external `Positive` choices retain four history statements, complete the
  mandatory update and close with both greeting flags set.
- An external `Exit` retains the initial history statement and closes with both
  greeting flags clear.

The compared positive update also matches the processor controls and counters;
the local player's phase and turn update counters advance from 1 to 2. Native AI
planning-handler differences remain recorded and have a finite source-qualified
policy exclusion. Represented gameplay fields and reached RNG match; the global
RNG inventory remains incomplete.

These cases have no cities, active deals or routes, positive pledges, active
third-party diplomatic relations or nonzero third-party grievances. Capital
hospitality/reveal branches, positive pledge accumulation, active third-party
opinion calculations and other unqualified effects remain unsupported. The
comparison does not establish complete diplomacy, arbitrary content or full-turn
equivalence. See [overall status](status.md) and the [target profile](targets.md).
