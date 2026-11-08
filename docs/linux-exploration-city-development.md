# City development after retained exploration

This bounded Linux profile continues the [first-city sequence](linux-exploration-first-city.md) on the **same persistent Rust Session**, through ordinary culture expansion, growth, Pottery and Monument completion, Code of Laws, external policy choices and one following round. Complete Civ VI gameplay, arbitrary actions and unrestricted turns remain unimplemented. The target is the [pinned Linux XP2 profile](targets.md).

The accepted exploration prefix and original Settler founding at **[18,11]** remain unchanged. There are no intervening resets, reloads, reimports or native AFTER-state inputs. Player 0 chooses every reached research, production, policy and unit response externally; NPC policy boundaries retain the recorded external responses.

| Completed round | Reached result |
| --- | --- |
| 6 | First city exists, with externally selected Monument |
| 13 | Ordinary culture expansion completes through the canonical territory and sight owners |
| 15 | Pottery completes; the following external technology choice is Animal Husbandry |
| 18 | Monument completes; the empty production queue receives a legal external Granary choice |
| 19 | Code of Laws completes; external requests slot Discipline and Urban Planning |
| 20 | Following ordinary round returns player 0 to a decision boundary at turn 21 |

Chiefdom is retained. This sequence does not grant later civics or select an unavailable government. After Code of Laws, the following civic choice is externally selected Craftsmanship. Research successors are chosen only when the corresponding queue is empty.

At turn 21, City `65536` has population **2**, stored food **3840 Q8** and Granary production **3840 Q8**. Discipline gives the original Warrior `131073` and Scout `196610` **+5 combat against barbarians**. Urban Planning contributes **+1 Production**, and the following round adds **1792 Q8** Granary production. Both policy requests and the following-round active policy flags, complete Culture component, city yield changes, production queue and unit combat modifiers match the native observations. The next unit orders remain external.

Composition reuses the existing stock culture-annexation and building-completion services. City creation, modifier instances, requirement records, territory, sight, History/Eras and Script callbacks retain their canonical owners. The building callback preparation accepts a Player before its first City exists; actual completion resolves the live City. Immutable building requirement definitions join the shared factory catalog with conflict checks. No second mutable registry or captured endpoint repairs the state.

Development comparison passes at public `bbec8d68` / private `21114e91`. Private receipt `ea40845a`, clarified by `81005e94`, records the committed source pins, executable, commands, input hashes and actual result. Optimized closing receipt `6c6b6810` passes this twenty-round sequence and the prior first-city regression on one executable at the same compiled source pins. The older Urban Planning fixture stops before policy execution because its current growth modifier owner is unavailable; that incompatibility remains recorded and is not a passing regression. The unchanged public Rust suite is reused from `a948ea43`. All nineteen intermediate complete Techs/Culture comparisons match for all four represented players. Final typed gameplay fields, full City/District state, represented requirements and modifiers, Map, unit rosters, encounters, stored/live Climate, ordered event history and all twelve event rates match. The CityCenter's live unsaved deletion flag also matches `false`; its serialized absence remains separately reported. Raw transport and source-qualified planning differences are retained.

Final captured GameSync matches **778 calls / seed 2566472039 / zero resets**. This is captured-stream agreement for this sequence, not a complete RNG inventory or complete-game equivalence claim.

The canonical private capture is `artifacts/linux-exploration-city-development/native-1`, configuration `9caa415039c978138e6c25f876e3c064cb3ed5260d03b8fefc6114bb9663f2a0`, seal `f1522adeb986344725c5b9d5227dae7775da32ca1e700c5bde4ca4fd120a116f`. Initialization retains GameSync seed **591945** and map seed **917431**. The final input manifest is `ae5866c9316d9548ee61a241fe572abbe3db6979afd13c757d85f8f0ebc6f950`. Production remains standalone Rust; private readers and the original engine are validation tools.

The optimized run measured **0.637 ms** for its initial Scout `Session::move_to` request. Its **380.850 ms** replay interval includes external requests and trajectory/policy checkpoint comparisons, and excludes initialization, input/rule preparation and the final full native comparison. It is not a pure twenty-round engine benchmark.

Other development states, later arbitrary actions, different policy effects and unsupported reached event branches remain outside this profile. The actual turn-21 city has no forced worker assignments; a retained external citizen lock/unlock sequence with these building/policy effects is a concrete next action interaction, and is not established by this comparison.
