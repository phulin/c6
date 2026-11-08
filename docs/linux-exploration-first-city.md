# First city after retained exploration

This bounded Linux profile continues the [flood-expiry sequence](linux-flood-expiry.md) through the original Settler's first city and one complete ordinary city-upkeep round on the **same retained Rust `Session`**. Complete Civ VI gameplay, arbitrary actions and unrestricted turns remain unimplemented. Acceptance uses the [pinned Linux XP2 target](targets.md).

The accepted prefix is unchanged: wonder discovery, external Alpine promotion, original village entry and Bronze Working boost, the Potomac flood and its following expiry round. At turn 6, player 0's original Settler `65536` remains at **[18,11]**. The original engine accepts its ordinary FoundCity operation there. Rust checks that location against its live Settler and executes the same parameterless action through the existing stock founding services. No state is injected, reimported or reloaded between actions.

The resulting first city and CityCenter have logical ID `65536` in their respective component domains. Naming, capital/Palace effects, territory, citizens, modifiers, requirements and Settler consumption retain their canonical owners. An external policy selects the original-rule-legal **Monument**. Both original Warrior `131073` and Scout `196610` survive the next ordinary round, which returns player 0 to a decision boundary at turn 7.

Pottery was selected externally in the first exploration round. Code of Laws was already queued in the original BEFORE state. The post-founding policy observes both existing queues and leaves them selected; it does not manufacture a replacement decision. Their complete Techs/Culture components match native observations for all four represented players before village entry, after entry, after rounds 4 and 5, and after the new city-upkeep round.

| Turn-7 development | Native and Rust Q8 |
| --- | ---: |
| Monument production progress | 1024 |
| Stored city food | 512 |
| Pottery research progress | 640 |
| Code of Laws civic progress | 332 |

These are one-round progress values; this profile does not complete the Monument.

Composition retains one History, Era, territory, Script, modifier and Unit-lifecycle owner. Identical initial observations join without resetting state; conflicting observations still refuse. The lifecycle input union uses captured Unit/component identities and preserves existing ability wrappers. It includes both actors needed by this trajectory: the Scout's earlier permanent village modifier and the Settler's later destruction.

Development comparison passes at public `bbec8d68` / private `1dfec773`; private receipt `7b4f5135` records the actual commands, source pins, executable, inputs and result. Full typed City, District and represented economy fields match, alongside compared gameplay/core state, unit rosters, requirements, selected modifiers, encounters, Map, live/stored Climate, ordered event history and all twelve event rates. The CityCenter's unsaved deletion flag is independently read from the existing native AFTER memory image and matches `false`; its serialized absence remains separately reported. Raw planning and transport differences remain preserved.

Final captured GameSync matches **746 calls / seed 3670766471**, with zero resets. This is captured-stream agreement for this sequence, not an exhaustive RNG inventory or complete-game equivalence claim.

The canonical private capture is `artifacts/linux-exploration-first-city/native-1`, configuration `b45d670a2d3469ec4ba88e034abec94b7faa8953586a026bbb02864bff3dc319`, seal `6a69501dc752854777ca6957f3ce9cc504b9474f2d43374cc783285efd302ea2`. Initialization retains GameSync seed 591945 and map seed 917431. Its 113-packet input manifest is `d127b09e52f1b80b009b6389eac9b8183477ab2e7f68cf1eb029c4e49e093095`. Native AFTER data is comparison-only. Production remains standalone Rust without an original-engine dependency.

Other founding states, later arbitrary development, positive drought/eruption effects, city flood damage and lethal flood damage remain outside this profile. Unsupported reached transitions remain explicit failures.

One optimized executable at the same compiled pins passes this sequence, the prior five-round flood-expiry case and a compatible second-city founding plus ordinary-round regression; private receipt `e236829b` retains its identity, commands and actual results. No first-city request timing is claimed.

Closing checks at public `bbec8d68` pass **1,843 Rust tests**, strict all-target Clippy and a standalone workspace build from committed public-only sources without private siblings; private receipt `a948ea43` retains the actual commands and results. No C ABI check or remote publication was performed.
