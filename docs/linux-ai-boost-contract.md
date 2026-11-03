# Linux AI BoostManager wire contract

The pinned Linux `libGameCore_XP2.so` SHA-256 is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
This is a static contract for the nested BoostManager record used by current
CultureAi version 15. It does not implement gameplay state, actions, turns or
native loading effects.

Reproduce the public, read-only audit:

```sh
python3 tools/linux/ai_boost_contract_audit.py --output /tmp/ai-boost-contract.json
cmp /tmp/ai-boost-contract.json evidence/linux/ai-boost-contract.json
```

The audit gates the exact module hash, 27 complete method hashes, 21 call/tail-call
and concrete PLT relocation bindings, four virtual bindings, version immediates,
and matching target DWARF layout digest. It starts no inferior process. It writes
only the requested JSON report; no raw binary, disassembly or DWARF dump is
archived. Supplied Linux ELF symbols, bytes and DWARF are the evidence source;
external reference material is not reproduced.

## Ordered current grammar

All words below are four bytes in the Linux little-endian stream. The native
integer stream leaves request four raw bytes. The complete record size is
`16 + 8 * managed_count`; there is no additional per-record version.

| Order | Source identity | Wire representation | Meaning |
| --- | --- | --- | --- |
| 1 | Local version | signed `i32`, writer emits `1` | Independent of CultureAi version `15` |
| 2 | `m_player->GetID()` | `PlayerTypes` through signed `i32` | Player identity, not pointer bytes |
| 3 | `m_allowedActiveBoosts` | raw `u32` | Allowed active boosts; distinct from serialized record count |
| 4 | `m_managedBoosts.size()` | `u32` | Number of following records |
| 5a, repeated | `BoostManagement::BehaviorTreeControlId` | raw `u32` | DWARF `GameCore::IDType`, typedef `unsigned int` |
| 5b, repeated | `BoostManagement::BoostUniquenessHash` | raw signed `i32` | Preserve signed hash bits and source order |

The nested type is concrete, non-template `GameCore::AI::BoostManager`. Its
records contain no CivicTypes or TechTypes value, definition-index conversion,
definition-collection count, Q8 value, or bool. `BoostUniquenessHash` is not
serialized through `WriteIndexToHash` or any definition registry. The spelling
“hash” does not authorize treating it as a Civic or Technology definition hash.
There is no deduplication, sorting, hash remapping or count-to-allowed validation
in these stream methods.

Matching DWARF gives a 64-byte native object: player pointer at `0x00`, managed
vector at `0x08`, support-function vector at `0x20`, allowed-active count at
`0x38`, followed by padding. Each managed record is eight bytes. Neither support
functions, capacity/allocation metadata nor pointer bytes appear on the wire.

## Concrete bindings

CultureAi's writer `0x2028c60` calls its BoostManager tracked wrapper at
`0x2028cbf` (PLT `0x1f9d220`, body `0x20357e0`), using the tracked member at
CultureAi offset `0x248`. Its reader `0x2028e20` calls at `0x2028ea4` (PLT
`0x1e24ee0`, body `0x2035bf0`). The tracked object's value is at offset `0x10`.
Its actual vtable `0x3a081b8` selects load `0x2042fe0` and save `0x2043050`.
Those tail-call the exact nested reader `0x1fc9630` (197 bytes) and writer
`0x1fc9580` (173 bytes).

Research's current writer `0x2183d20` and reader `0x2183e40` independently prove
use of the same concrete nested methods. Their actual calls at `0x2183df4` and
`0x2183fcc` select Research wrappers `0x218a590` and `0x218a460`, member offset
`0x1a8`; Research vtable `0x3a201a8` selects load `0x218bfe0` and save
`0x218c050`. These tail-call the same nested BoostManager methods. This proves
reuse of this nested wire contract; it does not prove reuse of CultureAi's
surrounding Civic analysis grammar for Research's Technology analysis grammar.
Full method names, sizes, SHA-256 values and relocation targets are recorded in
[evidence/linux/ai-boost-contract.json](../evidence/linux/ai-boost-contract.json).

## Read, reset and live effects

The BoostManager reader consumes the signed version into a local and never tests
it. Current-version-only codecs may explicitly support version 1; the native
reader's permissiveness does not establish other version semantics.

The native reader resolves the player through `Player::Manager`, overwrites the
player pointer and allowed-active count, reserves the managed vector and appends
one record per declared count. It does not clear existing records. The tracked
CultureAi load directly calls this reader, without invoking reset. Loading into a
fresh object and loading into a populated object therefore have different vector
postconditions. The separate reset method `0x20437e0` constructs a default manager
and move-assigns it; that operation must not be silently attributed to load.

After appending, reader calls `Initialize` at `0x1fc9700`, passing the resolved
player and allowed-active count. Initialize assigns those fields, reserves
managed capacity, and checks `GlobalAi::HasAiPersonality`. With a personality,
it obtains leader/civilization/handicap context, conditionally executes the
leader boost-support query and invokes support-function setup. These live,
configuration-dependent support effects are distinct from the wire fields and
are not reproduced by a typed record decoder. No RNG exclusion or planning-only
exclusion follows from the AI namespace or absence of RNG in this bounded method.

## Validation boundary

This research task used no native execution or owner-cut fixture comparisons.
The JSON intentionally reports an empty fixture list. No observed managed count,
player population, default count, or native save byte interval is claimed. A
future native oracle must pin rules, enabled DLC/mods, configuration and initial
object state, then compare affected state including the appended vector and
support setup effects.

Complete Player/world import, gameplay state, action execution and turns remain
unsupported. Unknown save bytes remain unclassified and unmasked; the recorded
raw Save-to-Load difference remains **89,198 bytes**. Static grammar closure is
not full native loading or gameplay equivalence.
