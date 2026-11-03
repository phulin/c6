# Direct Core prefix through first Player Units and Trade

The committed source reader now includes the first Player's complete Treasury, Units table and trailing Units fields, scalar Eras and Trade. The explicit next boundary is **Player.GreatPeoplePoints**; all 43 Player entries remain unfinished. This is wire-data import, not initialized gameplay state or action/turn support.

The complete source composition is `b7bd570`; repaired Treasury/Units codecs are accepted in `d0add45`, and the earlier Units/Eras composition is accepted in `c2802c3`. The complete Trade codec has its own `419c616` isolated source audit; its independent review remains separate. This producer source/driver is committed before any oracle run and requires a single isolated committed source archive, verified against Git for every recorded Rust/Cargo and Python input.

Owner validation of both original streams gives dynamically computed cursor **5,207,547**, post-Map bytes **511,683**, post-Map items **72,346** and **36 first-player Units**. The native Units end is an internal check at **5,191,951**. Eras has no direct owner hook: its 12-byte current record is checked between Units.end and the matching-parent Trade.start. These positions are comparison outputs, never parser constants.

The driver compares the entire typed prefix byte-for-byte and tests exact/one-short shared and Player byte/item ceilings. Unit slots, full IDs, raw embedded owners, locations, allocator words and complete record hashes are independently projected from native comparison cuts. The library receives only the full raw Core; it never receives those cuts, fixture offsets or byte-pattern hints. Inherited Unit handler/version/variant gates remain unchanged.

Unit free-pass sequence length has a proven signed native count. The envelope now rejects negative/high-bit counts before allocation and checks encoder length fits i32. Envelope-owned allocations use checked/fallible reservations; nested Unit decoding retains its previously accepted Vec allocation behavior after its original count/wire checks. No universal fallible-allocation claim is made.

Producer metadata follows after the isolated run. Current configuration and separate Game.DiplomaticRelations authorities are not inferred from Player records. Definition/domain remapping, native initialization/reset/callbacks, complete logical state and all gameplay RNG remain required. Unknown tail bytes remain borrowed; the **89,198-byte** raw difference stays unmasked. Old producer profiles remain immutable.
