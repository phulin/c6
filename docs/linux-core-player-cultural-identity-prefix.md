# Linux Core import through the first Player's CulturalIdentity

The direct Rust `core_stream::ThroughFirstPlayer::decode` API now composes the supported byte-zero Core prefix, complete Map, Game.Religion and the first Player's current wire components through CulturalIdentity. It returns typed data, its consumed cursor, item charge and an untouched borrowed tail. Python offsets and original owner traces are not library inputs. This is partial saved-state import; all 43 Player entries remain unfinished, and complete initialized world, gameplay RNG, actions and turns remain unsupported.

This extends the separately preserved [Trade profile](linux-core-player-units-prefix.md) with GPP, Influence, Quests, WMDs, Improvements, Governors and CulturalIdentity. Every component consumes its actual prefix under remaining shared and Player byte/item credit. Unsupported versions and forms reject explicitly; no unframed unknown payload is skipped. The next source-ordered component is Heroes. Native desktop/UI admission is not a prerequisite for this gameplay-only library, but real gameplay dependencies and native Read initialization effects still require faithful reconstruction.

The [producer audit](../tools/linux/core_player_cultural_identity_prefix_audit.py) gates the supplied Linux XP2 core and host hashes plus 1,513 method fingerprints. It compares the new [Rust driver](../crates/c6-differential/src/bin/c6-core-player-cultural-identity-prefix-candidate.rs) against independent original Save evidence. GPP, Influence, Quests and CulturalIdentity have native owner cuts. Outer WMDs, Improvements and Governors do not: WMD/Improvement framing is derived from source-proven version/count schemas, and the fixture comparator brackets Governors at the genuine following CulturalIdentity entry. Nested Governor instance cuts and the focused component audits provide additional evidence. Those comparison boundaries are never supplied to the production decoder. The computed stop must also match the genuine Heroes entry with the same native parent as CulturalIdentity.

Both 80,657,462-byte originals consume 5,213,659 bytes, including 517,795 post-Map bytes and 73,653 post-Map items (2,031,749 total items). The first partial Player contains 35 Formations, 11 Cities, 72 Districts and 36 Units. CulturalIdentity adds exactly 60 bytes and zero dynamic items. The two post-Map prefixes are byte-identical; their borrowed 75,443,803-byte tails retain different hashes. The complete raw inputs still differ in **89,198 bytes**, with zero bytes masked.

Checks include full prefix decode/re-encode, typed earlier state/Unit identity and allocator-sequence projection, all fourteen signed CulturalIdentity fields, exact and one-short shared/Player byte/item limits, and length-changing metadata/City-tail carriers. The aggregate item budget remains separate from component-specific typed-allocation policies; this profile does not claim an allocator-independent complete heap budget. Governors preserves raw slot words and ordering; transport acceptance does not establish native container validity or native Read behavior.

The producer is built from one committed Git archive, with every recorded Rust/Cargo and Python source checked against that commit before compilation. Earlier producer archives and reports remain immutable. The final report and frozen source identity are recorded in [the evidence](../evidence/linux/core-player-cultural-identity-prefix.json). Reproduce that closed profile with:

```sh
python3 tools/linux/core_player_cultural_identity_prefix_audit.py \
  --frozen-profile target/frozen/core-player-cultural-identity-prefix/0874f6e8f105c7dc82963fb2858084d64ea47a7d \
  --output /tmp/core-player-cultural-identity-prefix-replay.json
```

A new build instead requires an isolated committed archive and explicit `--source-commit` plus `--repository`; the audit refuses to build its candidate in the mutable shared checkout. Original game execution is false. Saved hashes, signed identities, counts and ordering are retained, but constructor/reset/remapping/subscription effects, live membership, domain rebuilds, unsaved fields and complete RNG are separate work. Retaining the remaining bytes neither classifies them nor permits advancing them correctly. Reference discovery attribution remains in [reference.md](reference.md).

Final producer source: `0874f6e8f105c7dc82963fb2858084d64ea47a7d`; candidate SHA-256: `0a905d03e6e1c90d3b74c542de8c08c6cea0d65ffc396a12d7ae89014df50742`. All 79 Rust/Cargo inputs and 177 Python audit inputs were verified against that single Git commit before the closed snapshot was copied.

Validation in the same final isolated source archive passed `cargo test --workspace`, strict workspace Clippy and the C ABI smoke check. The actual two-stream prefix corpus separately covers the ignored fixture-dependent source test. These checks execute Rust/C candidate code against existing native Save outputs; they do not execute the original game or reproduce its load-time side effects.

Independent closure is recorded in [the combined prefix review](linux-core-player-governors-prefix-review.md), commit `69a5b71`. It verifies both complete prefix cases, typed native projections and all 256 archived source identities; later Heroes/AI progression is a separate source profile.
