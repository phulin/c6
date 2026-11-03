# Direct Core prefix through first Player Techs

Source checkpoint: `core_stream::ThroughFirstPlayer` now composes all known schemas through the first Player's complete Techs component and returns the explicit **Player.Treasury** boundary. The source implementation is `bbc689a`; independently accepted composition/codecs are recorded by `f8f721d`, `8e303f1` and `9cbc3fd`. This still leaves all 43 Player entries unfinished. No complete world, native Read application, gameplay action or turn support is claimed.

The separate driver and audit are committed before the producer run. The audit refuses a shared-workspace build: it requires an isolated archive, an explicit source commit and a read-only repository used to verify every Rust/Cargo and Python audit source blob against that commit. Asset modules and canonical saved-record inputs are independently hash gated. No posthoc recovery from a mutable tree is permitted for this profile.

The completed independent source proof on both original streams gives absolute cursor **5,007,288**, post-Map bytes **311,424**, dynamic post-Map items **46,881**, and total items **2,004,977**. Treasury's native entry matches this cursor. The earlier internal Resources end **5,002,571** remains checked. Entire post-Map SHA-256 is `3c95e4082d1183ad429e0de654ee945984b7766d978e99723cfb607a3c04cfeb`. These are validation outputs, never parser constants.

The driver adds exact shared/Player byte/item limits and one-short rejection checks to the whole-input byte roundtrip. The audit independently compares every new component's complete original bytes plus acquired-tech hash/bool pairs. Native owner cuts are validation-only and never enter the library. Unknown tail bytes remain borrowed and unmasked; the full original Save→Load difference is still **89,198 bytes**.

The isolated producer now passes. Source commit is `27e51ee11ee359ea6b4695539f35a4ea51e28766`; candidate SHA-256 is `eaa764103859146eacd82ec22bc353f1a9193e8cecba089b886d0e9c56be00ff`. The [report](../evidence/linux/core-player-techs-prefix.json) records 66 Rust/Cargo source files, 151 Python audit sources and 1,023 target method fingerprints. Every recorded source blob matched that single commit before build and again before freezing. The exact candidate/report/source set is local at `target/frozen/core-player-techs-prefix/27e51ee11ee359ea6b4695539f35a4ea51e28766/`. No shared-source recovery occurred.

Both original streams pass the entire post-Map byte roundtrip, acquired-tech projection (77 pairs), saved-alive bridge, exact shared/Player caps and one-short failures. Source-only 20-byte metadata and four-byte Cities-tail insertions relocate the complete downstream boundary while preserving untouched tail digests. The remaining borrowed tail is 75,650,174 bytes; its different raw digest on each original is preserved.

For a fresh source build, create a git archive of that commit and invoke its audit from the extracted directory with explicit `--source-commit`, `--repository`, `--assets` and `--private-root` arguments. The audit rejects building from the primary repository. Historical replay can use:

```sh
python3 tools/linux/core_player_techs_prefix_audit.py --frozen-profile target/frozen/core-player-techs-prefix/27e51ee11ee359ea6b4695539f35a4ea51e28766 --output target/linux/core-player-techs-prefix.json
```

 Existing Cities/Districts/Culture profiles remain immutable. Native player validity, configuration Team/SlotStatus, separate Game.DiplomaticRelations, definition remapping, native Read fixups and all gameplay RNG remain independent requirements. Allocation policies remain component-specific and explicit; addressable typed storage is not silently conflated with serialized bytes. The library executes no native AI policy choices or UI bootstrap.

The exact isolated source snapshot also passes the full workspace checks: 226 c6-core tests, the remaining workspace tests, strict all-targets Clippy and the C ABI smoke test. No shared-workspace test result is substituted for this snapshot.
