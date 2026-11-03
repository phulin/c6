# Linux Player Treasury wire boundary

The pinned Linux XP2 `Player::Treasury` writer is `0x2b99260` (234 bytes), reader `0x2b99350` (544 bytes). Matching DWARF identifies the fields below. The writer loads local version **10** from `0x2f6937c`. Player's writer calls Techs at `0x2b4f0d3`, Treasury at `0x2b4f0e6`, then Units at `0x2b4f0f9`. The observed first record range is a corpus observation, never a parser constant.

`player_treasury::Treasury` preserves this little-endian grammar:

1. `u32` local version, exactly 10.
2. Ten signed 32-bit words: gold balance Q8; gold rate change Q8; cached net income excluding trade Q8; bankruptcy amenities; maintenance discount per unit; unit upgrade discount percent; maintenance discount per WMD modifier; levied unit upgrade discount percent; cached net income Q8; gold interest percent Q8.
3. `u32` count, then ordered `(u32 Improvement definition hash, i32 raw Q8)` pairs for gold dispersal change.
4. `u32` count, then ordered `(u32 first, u32 second)` pairs for gold diplomatic deals. These are unsigned native pair members, without inferred player/index/hash semantics.

No booleans, floating-point words, or nested local versions occur. Q8 words remain raw signed integers; no host floating conversion is involved. Array writer `0x1fe6820` emits the definition count and each hash/value pair in definition order. Sequence writer `0x2b9fee0`, count carrier `0x2b9ff90`, pair writer `0x2b9ffd0`, sequence reader `0x2b9fd80`, and pair reader `0x2b9fe60` establish the remaining framing.

Strict decode rejects trailing bytes; prefix decode returns consumed bytes and optionally aggregate item charges. Each array/vector element costs one item. Scalar fields/counts/versions cost bytes only. `encode_with_items` returns identical charges. Every read checks aggregate wire-byte limits before reading. Every vector checks the remaining item count, full minimum wire size and available input, checks its allocation layout, then uses a fallible exact reserve. Storage is checked separately from the wire cap; no cumulative typed-allocation cap is incorrectly compared to serialized bytes. These fixed-size eight-byte elements also bound total requested element storage by consumed wire bytes.

Native reading is separate from transport. Current reader invokes FAutoVariable edits and archive machinery, handles historical additions with version thresholds 2–10, and uses `ReadIndexHashArrayVersioned` (threshold 4). The hash-array reader `0x1ffbdd0` remaps Improvement hashes and consumes unknown-hash values into a temporary. The Rust codec retains all pairs and their order; it does not claim live registry remapping, Reset, historical-version migration, callbacks, clamping or gameplay effects. The current reader contains no explicit Treasury clamp or Reset call. Owner pointers and archive metadata are not serialized fields.

The offline audit checks supplied module/host identities, native method bytes and bindings, DWARF, and both immutable Core corpora. Native owner cuts only select comparison records; neither codec nor production composition consumes cuts. Candidate builds use an isolated archive of an explicit committed source pin and record actual source hashes. Original game code is not executed. Source tests cover nonempty arrays, signed extremes, unsigned pair extremes, framing, version rejection, hostile counts, every truncation, and exact/one-short byte/item limits; the audit repeats truncation and budget tests for every original record.

This completes only the current Treasury wire grammar. Full Player/world import, exact gameplay state, actions and turns remain unimplemented. Units is the next Player component. Unknown save bytes remain unclassified and unmasked; the recorded raw Save→Load difference remains 89,198 bytes.
