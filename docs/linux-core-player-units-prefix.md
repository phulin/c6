# Direct Core prefix through first Player Units and Trade

The committed source reader now includes the first Player's complete Treasury, Units table and trailing Units fields, scalar Eras and Trade. The explicit next boundary is **Player.GreatPeoplePoints**; all 43 Player entries remain unfinished. This is wire-data import, not initialized gameplay state or action/turn support.

The complete source composition is `b7bd570`; repaired Treasury/Units codecs are accepted in `d0add45`, and the earlier Units/Eras composition is accepted in `c2802c3`. The complete Trade codec has its own `419c616` isolated source audit; its independent review remains separate. This producer source/driver is committed before any oracle run and requires a single isolated committed source archive, verified against Git for every recorded Rust/Cargo and Python input.

Owner validation of both original streams gives dynamically computed cursor **5,207,547**, post-Map bytes **511,683**, post-Map items **72,346** and **36 first-player Units**. The native Units end is an internal check at **5,191,951**. Eras has no direct owner hook: its 12-byte current record is checked between Units.end and the matching-parent Trade.start. These positions are comparison outputs, never parser constants.

The driver compares the entire typed prefix byte-for-byte and tests exact/one-short shared and Player byte/item ceilings. Unit slots, full IDs, raw embedded owners, locations, allocator words and complete record hashes are independently projected from native comparison cuts. The library receives only the full raw Core; it never receives those cuts, fixture offsets or byte-pattern hints. Inherited Unit handler/version/variant gates remain unchanged.

Unit free-pass sequence length has a proven signed native count. The envelope now rejects negative/high-bit counts before allocation and checks encoder length fits i32. Envelope-owned allocations use checked/fallible reservations; nested Unit decoding retains its previously accepted Vec allocation behavior after its original count/wire checks. No universal fallible-allocation claim is made.

The isolated producer passes on source `e7f2aab03db7473a4e9c5b5d29b9ab7c42678114`, candidate `b92dacb3429de641bf11656ca1cc95337c599827bee78e49e1adc6f52215676a`. Its [report](../evidence/linux/core-player-units-prefix.json), executable and all 69 Rust/Cargo plus 159 Python source files are frozen under `target/frozen/core-player-units-prefix/<source-commit>/`. All recorded inputs matched Git blobs from that one commit before build and before freeze; no mutable-tree source recovery occurred. The audit gates 1,306 original method fingerprints.

Both originals produce complete post-Map SHA-256 `9ec9d8abff9a6ca636ff27d7a255ed3499f6d74e76baf3cb91b1d3acdf6c5804`, absolute consumed cursor 5,207,547 and total items 2,030,442. The unparsed tail is 75,449,915 bytes, with each original's raw digest retained. Source-only metadata/settled-Continent insertions relocate downstream positions without masking any bytes.

An initial comparison-only checker error treated Unit.ai_goal as a scalar rather than its actual PlotCoord. It was caught by the independent identity/location projection; source `e7f2aab` fixes the four-byte discrepancy. The production byte roundtrip already passed, but no producer acceptance was claimed for the failed `206a4ca` attempt. The corrected snapshot was built and validated afresh.

Replay this immutable profile with:

```sh
python3 tools/linux/core_player_units_prefix_audit.py --frozen-profile target/frozen/core-player-units-prefix/e7f2aab03db7473a4e9c5b5d29b9ab7c42678114 --output target/linux/core-player-units-prefix.json
```

A fresh build requires an archive of the exact source commit and the audit's explicit repository/assets/private-root arguments. Trade codec and final metadata independent acceptance remain separate from the already closed `c2802c3` Units/Eras source review.

 Current configuration and separate Game.DiplomaticRelations authorities are not inferred from Player records. Definition/domain remapping, native initialization/reset/callbacks, complete logical state and all gameplay RNG remain required. Unknown tail bytes remain borrowed; the **89,198-byte** raw difference stays unmasked. Old producer profiles remain immutable.

The exact isolated producer snapshot passes the full workspace suite (240 c6-core tests passed, one fixture-dependent test intentionally ignored), strict all-targets Clippy and the C ABI smoke test. The ignored full-input case is covered separately by both hash-gated producer fixtures.
