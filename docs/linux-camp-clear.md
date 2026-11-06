# Barbarian camp clearing

A normal queued `Session::melee_attack` MoveTo can kill a wounded camp Scout,
enter its plot and clear the camp in the bounded stock Linux profile. The
external policy supplies the unit action. Complete Civ VI gameplay, arbitrary
camp configurations and full turns remain unimplemented.

Use the [ordinary melee setup](linux-melee-combat.md) and bind
`loaded_actions::stock_notifications::camp_clear::State` through
`Runtime::bind_camp_clear`. Build its definitions from the admitted rules and
provide the current improvement-destruction subscribers, history listeners,
quest owner, research definitions and culture inputs. These services borrow the
same live visibility, script, modifier, history and research owners used by the
Session. `melee_attack::stock::Damage` must retain the current sight definitions
for the Scout's actual SeeHidden effect removal.

The accepted case creates a real stock camp with its original sole Scout,
sets that defender's damage to 90 before the action and executes ordinary melee.
It matches the original Linux target for dispersal gold, camp and area removal,
tribe bookkeeping, player camp statistics, Military Tradition inspiration,
both history moments, affected units and visibility, shared requirements,
modifier lifetimes and ordered GameSync draws. No turn is advanced.

Positive Initiation Rites faith/healing bonuses, resource-bearing camps, Clans
mode, active camp quests, replacement improvements and additional subscribers
remain unsupported. The new tribe death continuation covers the reached lost
Scout branch; other victim and formation branches return explicit errors.
Reached unsupported effects preserve earlier updates and freeze the Session.
This comparison establishes one complete action profile, not arbitrary combat
or complete RNG coverage across the game.
