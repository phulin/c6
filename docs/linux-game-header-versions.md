# Linux Game19/22 header through Map entry

The Rust `CorePrefix` reader accepts exactly Game versions **19 and 22**, retaining the actual `game_version` in both `CorePrefix` and `OwnedCore`. Context4/5 retain their separate existing gates. All nested Rules, processor, variant and Map gates and shared resource budgets remain unchanged. Unsupported headers fail explicitly; there is no signature search, opaque skip, old-player migration or writer extension.

The [source audit](../tools/linux/game_header_versions_audit.py) gates the supplied Linux module SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`, full Game.Read body at `0x2d7aaa0` (2,540 bytes) and its bounded header through return from the Map.Read call at `0x2d7ab45`. The [report](../evidence/linux/game-header-versions.json) records body/header hashes and encoded direct-call targets, without publishing proprietary disassembly.

There is **no wire delta between Game19 and Game22 before Map entry**. At `0x2d7aac9`, the only header version comparison branches around Rules for signed Game versions below14. Both accepted versions read Rules, Game.Processor (including Player/Phase processors), current turn, max turns, local player and properties, then Map. No field is missing in19 and no default is synthesized. Reset and InitializeLocalPlayer are native runtime side effects; this wire parser does not reconstruct their execution. Map and all nested records still require their own exact supported schemas.

This conclusion is bounded to the header. Later Game.Read version conditions and old Player/OperationDef schemas are not established by it. The byte-zero reader continues to refuse unsupported nested versions, retaining no guessed state. Existing complete-state, gameplay RNG, action and turn implementation gaps remain; accepting a header does not establish native Load equivalence or full raw-load support. No new original-code execution or native parity oracle was run for this change.

Focused tests compare19/22 retained fields, exact Map boundaries and unchanged borrowed tail, exact/one-short shared caps, every prefix truncation, unsupported adjacent Game versions, and strict nested Rules refusal. Existing Context4 metadata/default and current22 tests remain intact.

Reproduce the static source audit:

```sh
python3 tools/linux/game_header_versions_audit.py \
  --output target/linux/game-header-versions-static.json
cargo test -p c6-core core_stream::tests
```

After committing the source, an optional raw diagnostic compiles a complete `git archive` preimage and a retained small Rust probe, checks the original Read-input identity, and reports the next explicit decoder boundary. It never invokes original game code and does not transfer any SavedOutput offset. The report records the source commit/archive, generated manifest/lock and executable identities; the source archive and probe remain under `target/linux/game-header-versions/`.

```sh
python3 tools/linux/game_header_versions_audit.py \
  --raw-input ../c6-ghidra/evidence/saves/xp2-gamecore-stream.bin \
  --output target/linux/game-header-versions-raw.json
```

Earlier raw failures and the 89,198-byte Save→Load difference remain historical evidence without reclassification or masking. This work is Linux-specific; it establishes no Windows parity or cross-platform behavior equality.

The retained raw diagnostic uses committed source `3cf410594d6ded1b7f446d3f21babd06202270a8` (a descendant of header implementation `47304cf`) and candidate SHA-256 `04b0a4a897b113c6e80c6b697566509e630feebada7119b208732bdc80a4508e`. On the original76,556,938-byte input `a66736f0423e9a59b11c77b1879b471d936fe3887342df5008940f763dbcc5aa`, the Game19 header passes and the next refusal is `Map(UnsupportedVersion { expected: 6, actual: 5 })`. The rejecting gate is the Map **Plots collection** version at `map.rs`'s `plots` record, rather than Map's outer v4 header. No Plot5 migration is implemented. The first-Player continuation is therefore not reached.

Validation: all14 focused Core stream tests pass; the full c6-core suite passes435 tests with1 existing ignored test; c6-core all-target clippy passes with warnings denied. These are Rust/source checks, not native Load parity.
