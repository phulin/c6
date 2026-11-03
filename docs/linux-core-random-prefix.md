# Linux Core wire prefix through Random

`core_stream::ThroughRandom::decode(bytes, shared, post_map_limits, binding, contexts)` composes the current Linux saved-wire owners in order:

```text
ThroughGlobalAi
  → Barbarian.Manager5
  → FreeCities.Manager3 (Instance5)
  → Game sync Random1
  → borrowed, unparsed Trade.Manager suffix
```

The target is Linux XP2 `libGameCore_XP2.so` SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`. This is a source-only composition milestone. Native Read/reset, definition remapping, callbacks, actual configuration relationships, complete RNG inventory/state and gameplay/actions/turns are unsupported. The serialized Random record is not a complete executable RNG/world import. Unknown tail bytes and the recorded 89,198-byte raw Save→Load difference remain unmasked.

The result owns the existing `ThroughGlobalAi` result plus typed `barbarian`, `free_cities` and `random` values, and reports aggregate `consumed`/`items_used`. `tail` borrows the original input beginning immediately before Trade.Manager. Each actual codec returns its consumed length and exact item charge; composition uses neither fixture offsets, scans nor opaque skips. Previous prefix and context-free APIs retain their behavior. Complete PlayerManager wire decoding still requires the existing explicit per-role snapshot provider on actual WasEverAlive branches; this extension supplies no default initialization context.

Whole-image SHA verification remains in the delegated PlayerManager boundary and includes the untouched suffix. The later owners do not rehash the image. The shared byte/item ceiling covers every consumed owner from byte zero. The explicit post-Map ceiling covers Religion, all represented Player records, GlobalAi, Barbarian, FreeCities and Random together. Each child receives the minimum of both remaining ceilings, using checked subtraction/addition. Random charges zero items, including when preceding components spend the exact item ceiling. Borrowed suffix bytes do not spend consumed-wire credit. Existing Map allocation/depth and child typed-allocation policies remain their documented separate policies; no exact aggregate heap bound or post-Map shared-depth accounting is invented.

The component contracts and their independent evidence are [GlobalAi](linux-game-global-ai.md), [Barbarian](linux-game-barbarian.md), [FreeCities](linux-game-free-cities.md) and [Random wire](linux-game-random-wire.md). Composition preserves raw signed ordinals/scalars, hashes, duplicate ordered entries, bool bytes and Random seed/call/reset/synchronous carriers. It performs no owner normalization or logical interpretation of raw bool storage. Existing attribution/license boundaries in those documents apply.

Two new synthetic tests use explicit arbitrary typed constructors, including nonempty Barbarian claims/camps, FreeCities instances/buildability vectors, duplicate hashes and raw byte197. They check typed component re-encoding, the exact borrowed Trade tail, aggregate exact/one-short shared and post-Map byte/item caps, each added owner's current-version refusal and every truncated suffix across the new owners. All 11 Core stream tests and strict core Clippy pass. No original code or native capture is executed or consumed for this checkpoint; an offline candidate/profile is separate coordinator-owned work after the source freeze.
