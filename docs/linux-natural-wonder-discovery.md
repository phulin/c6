# Scout natural-wonder discovery

This bounded Linux profile discovers a natural wonder through a legal Scout `Session::move_to`, preserves its positive experience and discovery effects, and completes one ordinary round on the same Session. Complete Civ VI state, arbitrary actions and unrestricted turns remain unimplemented.

The implementation uses current unit definitions, promotion class, experience modifiers, discovered-feature flags and subscriber order. It shares canonical experience, History, Eras, research, modifier and requirement owners with later upkeep. Discovery executes the existing pending movement-completion/report transition before content callbacks; ordinary movement and operation processing retain their source order. Every player choice remains external.

The fixture creates a Scout through original APIs before BEFORE, verifies that the selected natural wonder is still undiscovered, then submits one ordinary legal move. The Scout moves from `[16,24]` to `[17,24]`, revealing feature 26 at `[18,22]`. Its remaining movement is `768→512` Q8 and returns to `768` after the following round. Native and Rust movement endpoints agree.

Native BEFORE and following-round observations agree with Rust:

| State | BEFORE | Following round |
| --- | --- | --- |
| Scout experience / level | 0 / 1 | 10 / 1 |
| Can promote / experience locked / promotion notice needed | false / false / false | false / false / false |
| Selected feature discovered | false | true |
| Astrology boost / progress Q8 | false / 0 | true / 4864 |
| History moments | 0 | 1, first world discovery, score 3 |
| Owner's historic era-score cell | 0 | 3 |

The final Scout has no queued operation, no pending movement completion and operation timer zero. Immediate native XP was not separately sampled; the native XP comparison uses the following-round endpoint. The ordinary round reaches the next external decision for player 0 with six processor updates.

The wonder's separate discovery-rate era subscriber has rate zero in this fixture; History supplies the three-point historic score. Native Gossip constructs a temporary occurrence, then the actual empty visibility history prevents archival or delivery. Its construction and destruction have no gameplay counter or RNG effect. The current unit-wonder subscriber ring is empty, and the retained religion's relic count is zero. The qualified manual notification becomes semantic output. Native processing-automation player `-1` differs from observer `0`, so the host helper does not request player reactivation. Its UI auto-end-turn inhibition creates no extra choice in the headless external-policy model.

The original round records two activation guards, four deactivation guards and four unit deferrals. The city-defender selection guard is installed and has zero hits in this cityless fixture. No native player policy chooses a promotion or ends a turn.

Development and optimized replays match the represented gameplay state, requirements, core processors, global Religion, canonical GameSync and libc RNG. Other native RNG records are retained as observations, without claiming candidate equality for them. Raw component differences remain recorded, including separately classified unit property-wire annotations. The reports retain false full-action and complete-RNG-inventory flags. The same optimized executable also passes the existing Iron Mine and surviving-major-melee ordinary-round regressions.

One optimized Scout MoveTo takes **0.908 ms**. This measures the `Session::move_to` call and its reached callbacks, excluding import, definitions/provider setup, request decoding, path-search allocation, observations, native AFTER comparison, external policy selection and JSON output. It is not a complete-game benchmark.

The native capture uses public `e04335a0` / private `38ae41e8`, configuration `1e93903a0662d342964a5ac5b2f652e18b04d4a6b47fbe0b8a31153460485aec` and canonical capture `artifacts/linux-major-wonder-discovery/native-1`. Passing replays use public `49d89033` / private `06594d5b`; optimized receipts are retained at private `e7ef42a9`. Earlier materialization and replay failures remain preserved. Exact target/rules identities, input paths/hashes, commands, build profiles and actual results are recorded privately. Typed component/heap import does not require producing a Civ VI Save, and AFTER data only enters comparison.

Public checks at production `49d89033` pass 1,832 unit tests, two doc tests, workspace Clippy with warnings denied, the existing C smoke check and a committed public build without private siblings. Production remains standalone Rust; this adds no C ABI gameplay operation.

The separate [Scout Alpine profile](linux-scout-alpine.md) now exercises threshold enumeration, an explicit promotion and its subsequent hill effect. Random promotion options, nonempty gossip delivery, positive relic creation, positive discovery-rate era notices, added unit-wonder consumers, automation reactivation and circumnavigation remain outside this qualified profile.
