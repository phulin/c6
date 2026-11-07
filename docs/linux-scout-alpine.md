# Scout sight experience and explicit Alpine promotion

This bounded Linux profile follows a Scout on one retained Rust `Session`: natural-wonder sight experience makes promotion available, an external policy explicitly chooses Alpine, ordinary upkeep restores movement, and three further legal moves demonstrate Alpine's hill effect. Complete Civ VI state, arbitrary actions and unrestricted turns remain unimplemented.

The fixture creates the Scout through original APIs and sets legitimate near-threshold experience before BEFORE. It starts at level 1 with 5 XP; an actual discovery move from `[16,24]` to `[17,24]` supplies 10 XP. Native and Rust clamp XP to the 15-point threshold, lock experience and expose promotion readiness. Promotion availability is optional; the library does not select a promotion or introduce a forced decision.

`Session::unit_promotion_choices` returns legal type hashes in native definition order. The fixture supplies the Alpine hash explicitly to `Session::promote_unit`. Eligibility, prerequisites, healing, level/experience flags, movement and callbacks use the existing promotion action and canonical modifier, requirement, collection and Script owners. The action retains 15 XP, advances to level 2, clears readiness and experience locking, and exhausts movement. A duplicate request is rejected without changing the Unit or promotion reports.

Alpine's rules select `EFFECT_ADJUST_UNIT_IGNORE_TERRAIN_COST`, `Type=HILLS`, `Ignore=1`. The effect traverses current terrain definitions and writes the retained indexed ignore-cost booleans; it is not a Scout-ID or promotion-hash exception. Native removal writes the inverse of the stored boolean. Overlapping applications therefore follow ordered boolean writes rather than reference counting. The runtime setter preserves the typed hash projection and other terrain/ability cells.

The native and Rust trajectories agree:

| Boundary | XP / level | Movement Q8 | Hill cost Q8 |
| --- | --- | --- | --- |
| BEFORE | 5 / 1 | 768 | — |
| Discovery move | 15 / 1, promotion available | 512 | 512 |
| Explicit Alpine | 15 / 2, promotion earned | 0 | — |
| Following ordinary round | 15 / 2 | 768 | — |
| Move to `[16,23]` | 15 / 2 | 512 | — |
| Move to `[16,22]`, before final hill | 15 / 2 | 256 | 256 |
| Move onto hill `[16,21]` | 15 / 2 | 0 | 256 |

The independent native and Rust `MovementCostNoZOC` queries use the same penultimate plot, final hill and explicit query budget of 768 Q8 before and after promotion. Their uncapped cost changes from 512 to 256, so the effect is demonstrated independently of the final move exhausting the Scout's remaining movement. These query arguments do not change gameplay state.

Discovery also sights a tribal village at plot 938. The reached source path only localizes a manual notice and calls the existing notification Send overload. Its actual BEFORE default factory, old-record supersession and NotificationAdded consumers are qualified; the headless runtime emits `GoodyDiscovered`. The Scout does not enter the village or select a reward. This profile establishes no village-reward RNG behavior. The separate XP-changed host publication becomes semantic promotion-availability output after its actual synchronous consumers qualify; it does not earn a promotion or choose a policy.

Development and optimized replay pass for the represented gameplay components, core processors, requirements, History, research and observed GameSync/libc RNG. Raw observations and differences remain retained. Full-action equivalence and a complete RNG inventory are not claimed. Random proposed promotions, stored-promotion XP continuation, unsupported promotion effects and positive later-level callback branches remain explicit errors. See the [promotion API and limits](linux-unit-promotion.md) and [base wonder discovery profile](linux-natural-wonder-discovery.md).

The canonical native capture is private `artifacts/linux-scout-alpine/native-2`, using public `6e05900e` / private `9640207c`, configuration `8b3c86777891200ec39f51cebf269bf91dd90354c7dcc3b45a4c4d60f01a7bec`. Passing development replay uses public `48f180c7` / private `61bc778c`; the final optimized executable uses public `48f180c7` / private `cec65b61`, with receipts at private `6b2d0b9f`. The same final executable passes the unpromoted wonder, Iron Mine and surviving-major-melee ordinary-round regressions. Exact Linux target/rules identities, source pins, input hashes, commands, build profiles and actual results are retained privately. Earlier route, import and gameplay failures remain preserved. AFTER inputs are comparison-only; no Civ VI Save production or original-engine runtime dependency is added to production Rust.

One final optimized sample measures the discovery `Session::move_to` at **0.634 ms** and the explicit promotion request at **0.875 ms**. These exclude import, definitions/provider setup, external policy selection and endpoint comparison; discovery timing also excludes request decoding, path-search allocation and JSON output. The separately measured ordinary round takes 9.767 ms and excludes explicit SkipTurn orders. Individual approach, hill-move and cost-query timings were not recorded. These are bounded request samples, not complete-game benchmarks.

Final Rust checks at production `48f180c7` pass 1,834 unit tests, two doc tests, workspace Clippy with warnings denied, and a committed public build without private siblings. This feature adds no C ABI operation.
