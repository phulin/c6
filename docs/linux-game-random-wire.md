# Linux Game sync Random wire

`game_random_wire::Random` transports the current normal `GameCore::Random` record owned by `GameCore::Game::Instance::m_kSyncRandom` for Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. It reuses the existing Linux-native-validated `rng::native_serialization::SerializedRandomRecord`; no RNG algorithm or live-state conversion is duplicated.

The exact little-endian writer grammar is version u32 `1`, seed u32, calls u32, resets u32, raw synchronous byte, legacy-vector flag u8 `0`: **18 bytes**. All unsigned extremes and all 256 synchronous byte carriers survive transport. This record omits the live 32-byte object's vptr, lock count and padding. It does not establish a complete inventory of gameplay RNG streams or complete live RNG state.

Target Write/Read are `0x2d48a20` / `0x2d48ab0`; stream operators are `0x2d48a00` / `0x2d48a90`. Game writer call `0x2d7a8ac` follows GetSyncRandom and sits between FreeCities and Trade. Game reader call `0x2d7aced` follows EditSyncRandom and precedes Trade. The audit pins complete RNG method bytes, scalar bindings and parent boundary neighborhoods and verifies PLT relocations against the complete target module hash. Existing [Linux native Random parity](linux-random.md) independently validates this raw codec and isolated arithmetic/state; this audit executes no original code.

Both canonical 80,657,462-byte streams contain the complete Game-owned Random at `71027778..71027796`, directly under Game, with no nested owner cuts. Independent Python unpacking reconstructs all native bytes; the unchanged exported Rust crate then compares every decoded field and complete byte roundtrip. Owner offsets select comparison records only; the production parser consumes its own fixed grammar. The next owner is `GameCore::Trade::Manager`.

The bounded API exposes `decode`, `decode_prefix`, `decode_prefix_with_items`, `encode`, and `encode_with_items` using `unit::Limits`. It consumes exactly 18 bytes and zero collection-item credits. A byte cap below 18 is rejected; successful prefix decode preserves following owner bytes, while whole-record decode rejects trailing input. Encoding uses fallible reservation. All truncation boundaries, raw numeric extremes, version errors, every nonzero legacy flag, and exact/short byte caps are covered.

Native Read consumes but ignores its version word, resets the target before loading fields and preserves its existing lock count. Its nonzero legacy flag branch consumes a historical vector. This **current-writer transport** rejects versions other than 1 and nonzero legacy flags explicitly; it does not normalize their bytes or emulate native partial I/O, the legacy branch, Reset, archive dirtiness, object registration or other Read side effects. Existing low-level raw Random decoding retains its established ignored-version behavior.

Full gameplay state/RNG, actual save loading, world import, actions and turns remain unsupported. Every player decision must still yield to an external policy. No save bytes are masked or classified as planning-only; the recorded raw Save→Load failure remains **89,198 bytes**.

Reproduce the committed exported-source audit with no original execution or private writes:

```sh
python3 tools/linux/game_random_wire_audit.py --source-commit f82e791 --output /tmp/game-random-wire.json
```

Immutable validation identity and results are recorded in [the evidence](../evidence/linux/game-random-wire.json).

Validation used unchanged committed archive `f82e791`: all three actual module Cargo tests, strict all-target Core Clippy and both complete native field/byte comparisons passed. All 112 source input hashes matched Git before and after building; no source was injected or modified.
