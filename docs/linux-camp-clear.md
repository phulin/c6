# Barbarian camp clearing

A normal queued `Session::melee_attack` MoveTo can kill a wounded camp Scout,
enter its plot and clear the camp in the bounded stock Linux profile. The
external policy supplies the unit action. Complete Civ VI gameplay, arbitrary
camp configurations and full turns remain unimplemented.

Use the [ordinary melee setup](linux-melee-combat.md) and bind
`loaded_actions::stock_notifications::camp_clear::State` through
`Runtime::bind_camp_clear`. Build its definitions from the admitted rules and
provide the current improvement-destruction subscribers, history listeners,
quest owner, research definitions and culture inputs. Positive pantheon bonuses
also require the current normal-placement definitions/queries, complete
`FaithChanged` listeners and qualified faith report consumers in the camp inputs.
Bind the current capabilities on the reporting and readiness providers; a city
requires its current yield queries and actual unsaved District appeal rows for
boost publication. These services borrow the
same live visibility, script, modifier, history and research owners used by the
Session. `melee_attack::stock::Damage` must retain the current sight definitions
for the Scout's actual SeeHidden effect removal.

The accepted case creates a real stock camp with its original sole Scout,
sets that defender's damage to 90 before the action and executes ordinary melee.
It matches the original Linux target for dispersal gold, camp and area removal,
tribe bookkeeping, player camp statistics, Military Tradition inspiration,
both history moments, affected units and visibility, shared requirements,
modifier lifetimes and ordered GameSync draws. No turn is advanced.

The accepted Initiation Rites continuation awards 50 Faith and heals the attacker
from 66 damage after combat to zero, clamping the configured 100 healing request.
Its original setup founds the pantheon and sets attacker damage to 50 before the
captured action. The comparison also matches the faith callback, damage reports,
owned-city boost publication, history, affected state and ordered GameSync draws.
Reward values come from current player effects and admitted rules: faith uses
game-speed scaling, while healing is unscaled. Both follow gold within the native
nonzero raw-gold branch.

Resource-bearing camps, Clans mode, active camp quests, replacement improvements
and additional subscribers
remain unsupported. The new tribe death continuation covers the reached lost
Scout branch; other victim and formation branches return explicit errors.
Reached unsupported effects preserve earlier updates and freeze the Session.
These comparisons establish bounded complete action profiles, not arbitrary combat
or complete RNG coverage across the game.
