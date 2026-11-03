# Linux Player.Heroes saved wire

This boundary implements the current version-6 Heroes record from Linux
`libGameCore_XP2.so` SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
The parent Player writer calls it at `0x2b4f1b7`, immediately after CulturalIdentity.
This is saved wire transport, not complete Player import or gameplay execution.

The writer at `0x269b310` and reader at `0x269b3a0` establish this order:

| Member | Saved representation |
| --- | --- |
| version | little-endian i32, current 6 |
| heroes discovered | u32 count, ordered `{u32 HeroClass definition hash, canonical u8 bool}` entries |
| goody-hut discovery chance | i32 |
| player influence chances | u32 count followed by i32 values |
| player suzerain counts | u32 count followed by i32 values |
| hero discovery order | u32 count followed by versioned discovery-weight records |
| hero claims | u32 count followed by versioned claim records |

Discovery-weight writer/reader `0x269b4c0`/`0x269b500` serialize version 6,
HeroClass hash and signed i32 weight, totaling 12 bytes. Claim writer/reader
`0x269b540`/`0x269b5a0` serialize version 6, HeroClass hash, claim Era hash,
signed i32 claim turn and last-appearance Era hash, totaling 20 bytes.
Every version is independently checked. Legacy versions are unsupported.
Empty Heroes records occupy 28 bytes and zero dynamic items.

Enums written through `WriteIndexToHash` are definition hashes, including their
saved sentinel bits, not database ordinals. The codec preserves order, duplicate
hashes, all integer bits and complete saved fields. It does not resolve hashes
against a rules registry. Native Read maps hashes to current indices; missing
array hashes still consume their boolean into a temporary discard slot. That
live remapping is distinct from lossless saved transport.

The reader uses archive edit wrappers and version defaults. The two integer
vectors enter at versions 2 and 4, discovery order at 3, claims at 5, and the
last-appearance Era at 6. Constructors, archive dirtiness, parent linkage,
signal subscription and derived project defaults are not reproduced. Hero
choices remain external policy decisions; gameplay RNG remains in scope.

Decode performs an allocation-free validation pass before fallibly reserving
owned vectors. Every count charges shared item credit, validates minimum wire
bytes and checks aggregate owned-storage arithmetic. Encode validates the full
record before reserving output. Prefix APIs report exact bytes and dynamic
items for parent budget composition; exact decode rejects trailing data.

`tools/linux/player_heroes_audit.py` pins original code bytes, PLT bindings and
DWARF, then builds only a named committed Git archive. Canonical save owner cuts
are validation context: the preceding WMD, Improvements, Governors and
CulturalIdentity grammars locate Heroes within enclosing Player bounds. There
is no direct Heroes owner hook, so these are contextual prefix comparisons,
not a separately instrumented original writer oracle. No trace offsets enter
production decoding. The report records both corpora and exact source identity.
No original game code is executed by this audit.

The next Player boundary is conditional: `m_bWasEverAlive` at Player+0x278
(the value of the bool archive wrapper at +0x268) is saved as header boolean
index 1 by call `0x2b4ef3d`. The check at `0x2b4f1bc` jumps to `0x2b4f413`
when false, skipping the Unit/Feature/Continent boolean hash arrays and all
15 AI blocks. These blocks are present when the flag is true. Full Player/world import, exact gameplay state,
actions and turns remain unsupported. Unknown save bytes remain unclassified
and unmasked; the recorded raw Save→Load difference remains 89,198 bytes.
