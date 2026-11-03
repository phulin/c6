# Owned Linux Game.DiplomaticRelations

`c6_core::game_diplomatic_relations::GameDiplomaticRelations` owns actual
Game.DiplomaticRelations query values. Player.Diplomacy is a separate owner and
never supplies this state. This is a bounded current-version Read prefix and
query implementation; full relation initialization, setters, callbacks, actions
and turns remain unsupported.

The pinned Linux XP2 module is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
The [source audit](../evidence/linux/game-diplomatic-relations-source.json)
fingerprints the original Read, enum readers, definition remapping and queries.
No original code was executed for this change. Raw disassembly and saved owner
bytes remain in `../c6-ghidra/evidence/linux-game-diplomatic-relations/`.

`decode_prefix(bytes, unit::Limits)` supports version 16, retaining four bool
relationship vectors, the intervening integer/hash vectors, the WarTypes vector,
its following PlayerTypes vectors, the bool at owner+0x320 and the first
AllianceTypes vector. Each relationship preserves its original changed-turn
word. Bool wire records occupy five bytes; integer/hash records occupy eight.
Offset-labeled fields retain attribution where their gameplay meaning has not
been closed. `DecodedPrefix.tail` borrows every remaining byte unclassified;
this decoder neither frames nor excludes that suffix. Old-version defaulting and
remapping branches reject explicitly.

Saved enum values require original Read initialization. AllianceTypes Read
(`0x2bf6c00`) reads a saved hash and performs Find(hash), then obtains the found
Alliance definition's actual collection index; a miss returns -1. WarTypes Read
(`0x2d503a0`) validates a saved hash against WarDefinitions and returns the found
definition's type hash; a miss returns -1. Consequently decoding does **not**
automatically make these saved words gameplay authority.
`initialize_definitions(war_hashes, alliance_hash_and_index_pairs)` performs that
one-shot load step using complete admitted native-order definition inputs.
The actual Alliance Find (`0x206f1a0`) and War Find (`0x295cfb0`) first treat an
unsigned key below collection count as an ordinal, even when another definition
has that small numeric hash. Otherwise they search the unsigned hash vector.
Native insertion at lower_bound means the most recently inserted equal hash wins;
duplicate hashes remain valid. War initialization returns the resolved definition's
hash, so a small ordinal can canonicalize to a different number. Alliance
initialization returns the resolved definition's actual collection index, which
is distinct from the input key. Misses produce -1. Invalid definition inputs
reject before either gameplay vector changes. Raw hashes
remain in `read_prefix().saved_war` and `saved_alliances`. Until initialization,
queries requiring those vectors reject. `from_war_values` separately admits
already-initialized native metadata first words and retains unknown changed
turns and alliances. The caller must establish snapshot/configuration provenance;
these component APIs do not independently establish it.

Native IsAllied (`0x2983170`) and IsAtWar (`0x2983ba0`) return false for equal
players before accessing storage. Raw type getters similarly return -1 for equal
players. For unequal players, the signed wrapping index is
`min(a,b) - ((max(a,b)+1)*max(a,b) >> 1) + ((n-1)*n >> 1)`.
Complete vectors must have `((n-1)*n >> 1)` entries. Exactly -1 denotes absence;
other signed values are retained. Invalid unequal player slots and unsafe storage
indices reject explicitly instead of reproducing an invalid native memory read.
Owned initialized vectors remain authority across unrelated Unit mutations;
there is no global state revision check or speculative future revision binding.

The existing ff31 Core snapshot has 80,657,462 bytes. Its previously captured
serializer-owner trace attributes Game.DiplomaticRelations to
`[72493107,72688728)`, call 175316, parent 1, depth 1. The 195,621-byte owner has SHA-256
`393347b11100e83ca439a3967d851df8a543f55a006aa3b68b8a516098e9d34b`.
The current prefix consumes 121,005 bytes and leaves 74,616 unclassified bytes.
The actual player count is 64; each relevant vector has 2,016 entries. There are
26 saved alliance entries whose hashes differ from -1, across five hashes.
These are saved facts, not a native runtime alliance-type comparison.
The independent Python projection and Rust checker compare all 18,144 records
across nine vectors, including every original first word and changed-turn word.
The owner offsets are fixture attribution only, not a general Core parser index.

Reproduce the static audit and actual-record comparison:

```sh
python3 tools/linux/game_diplomatic_relations_audit.py --private-output ../c6-ghidra/evidence/linux-game-diplomatic-relations --output target/linux/game-diplomatic-relations-source.json
cargo build -p c6-core --lib
rustc --edition=2021 tools/linux/game_diplomatic_relations_check.rs --extern c6_core=target/debug/libc6_core.rlib -L dependency=target/debug/deps -o /tmp/c6-game-diplomatic-relations-check
/tmp/c6-game-diplomatic-relations-check ../c6-ghidra/evidence/linux-game-diplomatic-relations/owner.bin ../c6-ghidra/evidence/linux-game-diplomatic-relations/expected-records.bin
cargo test -p c6-core game_diplomatic_relations --lib
cargo clippy -p c6-core --lib -- -D warnings
```

The saved-record comparison report pins the earlier `1841167` source candidate;
it remains historical evidence for that decoder. The subsequent definition lookup
correction is covered by the focused source tests and updated static audit. It
does not claim a new native fixture comparison.

After the first Alliance vector, the current Read's explicit suffix only supplies
wrappers at +0x230, +0x258, +0x280, +0x2a8, +0x2d0 and +0x2f8, then returns.
It does not explicitly revisit War +0x190 or the first Alliance +0x208 wrapper.
Enclosing Game post-load and transitive callback equivalence remain unclosed;
this prefix alone does not admit those effects.

Tests cover reverse triangular order, signed sentinel behavior, preserved changed
turns, explicit missing authority, ordinal/hash collisions, duplicate precedence,
unsigned hash extremes, atomic definition admission and bounded
truncated/resource inputs. Full Core loading and bound rules admission remain the
containing loader's responsibility. Native alliance query parity remains a gap;
source-only query tests and saved-record comparisons do not close that gap.
