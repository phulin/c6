# Linux Military initialization and saved-array framing

The Linux framing condition is source-closed, but its genuine runtime inputs have
not been captured by this audit. `Military` version 14 omits three operation-type
hash arrays when the **live allowed-operation vector is empty**. No presence flag
is saved. Neither the active definition count nor saved `WasEverAlive` alone
resolves that condition. This is an initialization contract, not an implemented
initializer or an original Read oracle. Complete Player/world import, actions and
turns remain unsupported; the recorded 89,198-byte raw Save→Load difference stays
unmasked.

The authoritative target is supplied Linux `libGameCore_XP2.so`, SHA-256
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`.
The [static audit](../tools/linux/ai_military_initialization_audit.py) checks 29
complete method fingerprints, thirteen direct call/PLT/GOT bindings, five branch sites
and matching DWARF. Its [report](../evidence/linux/ai-military-initialization.json)
contains public identities and contracts, without raw disassembly. No original
code executes, no private artifacts change, and no future capture profile changes.
Discovery context remains attributed in [the existing Military audit](linux-player-ai-military.md).

## Exact condition

Let `P` be `Player.IsBarbarian() || Player.WasEverAlive()`, `L` the actual result of
`GlobalAi.GetLeaderType(player)`, `Q` the actual prepared leader-query pointer in
the standard OperationsHandler, `D` the count of the active AiOperationType
**definition collection**, and `A0` the allowed-vector size before this invocation.
On normal completion of `Military.InitializeLists` with the standard handler
callbacks installed by `GlobalAi.Initialize`:

```text
A = if !P:                         A0
    else if L == -1:               D
    else if Q == null:             A0
    else:                          D
operation_type_arrays_present = A != 0
```

`A0` must come from genuine object lifetime evidence or a typed prior-state
snapshot. It is zero after a fresh constructor or `Military.Uninitialize`; it
cannot be assumed zero merely because Read or Initialize is being invoked.

The constructor `0x2146900` constructs the three FAutoVariable vectors through
`0x214e1c0` and the zero-size vector constructor `0x1fe3170`.
`Uninitialize` (`0x2147880`) explicitly clears all three at
`0x214791b`–`0x2147952`. `Initialize` (`0x2147700`) stores its Player pointer,
initializes its archive and calls `InitializeLists`; it does not first clear those
vectors. Read (`0x2145620`) obtains the saved Player ID, resolves the real Player,
stores it at `Military+0x98` and calls `InitializeLists` at `0x2145664`.
It likewise does not clear those vectors first.

`InitializeLists` (`0x2145be0`) calls `HasAiPersonality` at `0x2145c02`.
That method (`0x20fb0d0`) is exactly the short-circuit predicate `P`.
False jumps to `0x2145fcb`, preserving all three prior vectors. This destination
continues with signal subscriptions; it is not a successful return that bypasses
all services. The proven dependencies are Player/game signals, archives,
GlobalAi handlers, configuration, active definitions and the gameplay database.
There is no UI prerequisite in this call path.

For `L == -1`, calls at `0x2145dc5`–`0x2145de1` assign `D` zero values.
For a known leader, `ExecuteLeaderQuery` (`0x203ac30`) checks `Q` before touching
the list. With a nonnull query it clears, calls the handler count callback, and
resizes before stepping query results. The standard count callback `0x2114210`
reads the same AiOperationType collection captured by handler Initialize
(`0x2103800`). Its list-initialization callback `0x2113f30` only returns. Query
rows modify elements in place; neither leader nor civilization rows resize this
vector. Even zero matched rows therefore produce size `D` when query preparation
succeeded. Null query preserves `A0`.

The current-operation vector is assigned zero values with size `A` at
`0x2145e95`–`0x2145ebf`, only on the personality branch. The per-war vector uses a
separate handler and query; it is not the presence guard. Consequently no general
three-vector-equal-size invariant follows for arbitrary prior or failed-query
states. Writer `0x2145506` and reader `0x21457cf` both test the allowed vector.
The hash-array helpers use the active definition collection, not the guard's
vector size, to determine their serialized entries. Safe save compatibility
requires the initialized arrays to cover that indexed collection; a nonempty
short prior vector cannot be promoted to a valid native save context.

`D` is `(definitions.end - definitions.begin) / 8` (`GetCount` at `0x2114220`,
`size` at `0x2114230`), counting the ordered active definition pointers. A SQL row
count, largest hash, maximum enum value or saved array's observed count is not a
substitute. The two original Save corpora's observed count of seven does not
supply `D`, `P`, `Q` or `A0` for another fixture.

## Genuine input contract

A resolver must accept attributed initialization data, rather than a caller
presence/preparation boolean. Its source inputs are:

- Module and matching host identity; fixture/save identity; actual configuration,
  gameplay rules database, enabled DLC/mods and definition ordering identities.
- The real Player ID and lifetime owning this Military; actual civilization-level
  ID, `WasEverAlive` byte and handicap. `IsBarbarian` compares the configuration
  civilization-level type to `0xd959ec24`. `WasEverAlive` reads `Player+0x278`;
  handicap is a signed enum value at `Player+0x2d8`.
- For nonbarbarians, configuration Player `GetLeaderTypeID`, resolved through
  `Types.GetIndex(LeaderTypes)`. For barbarians, `GetLeaderType` uses the cached
  `GlobalAi.s_barbarianLeader`, or scans ordered leader definitions for the first
  `IsBarbarianLeader` and stores its collection index. Preserve that cache's
  lifetime/rules identity; do not invent an ordinary leader for barbarians.
- The actual configuration civilization type ID returned by
  `Player.GetCivilizationType` (`0x2b304c0`), and actual handicap returned by
  `GetHandicap` (`0x1fcc600`). The known leader's definition supplies its leader
  type string. These determine query values, not the resized list length.
- Ordered active AiOperationType definitions with hashes/indices, and genuine
  prior vectors or proof of the constructor/Uninitialize-cleared lifetime.
- Standard `GlobalAi.Initialize` completion and the actual gameplay-database
  query construction outcome. At `0x20f7a3c` it initializes OperationsHandler
  using AiOperationTypes/OperationType and the active definition collection.
  `Factory.MakeLeaderQuery` (`0x21c2b80`) creates its query through the gameplay
  database virtual slot `+0x18`; merely finding table names does not prove a
  nonnull prepared query. Handler construction (`0x2100d40`) starts with null
  queries and empty callbacks, so construction alone is not handler initialization.

This closes the size dependency without implementing SQL row evaluation, leader
inheritance, OperationDef list construction, signal subscription or policy. Those
remain relevant to faithful full initialization and gameplay; they cannot be
silently replaced with empty objects to create a supposed native fixture.

## Attributed native snapshot alternative

A snapshot can avoid recreating those services for framing if it captures their
actual completed result from the same genuine Player and immutable rules/config
lifetime. It must contain **all three complete typed vectors**, their explicit
lengths and ordered `i32` values, active ordered AiOperationType identities, Player
identity, input/config/rules provenance and an attributed normal initialization
boundary. A count of seven or an uncorrelated pointer/boolean is insufficient.
This is a complete snapshot of the framing-relevant initialization result; it is
not a claim that the rest of Military/Player initialization has been reconstructed.

The matching DWARF and fingerprinted field methods establish this readonly ABI:

| Field | Object-relative address | Type |
| --- | --- | --- |
| owner Player | Military `+0x98` | native pointer |
| allowed vector value | Military `+0x158` | native `Vector<int>` |
| per-war vector value | Military `+0x180` | native `Vector<int>` |
| current vector value | Military `+0x1a8` | native `Vector<int>` |
| vector backing pointer | vector `+0x0` | native pointer to `i32` |
| vector logical length | vector `+0x10` | `u32` |
| OperationsHandler | GlobalAi `+0x25d0` | `Handler<int>` |
| per-war handler | GlobalAi `+0x2720` | `Handler<int>` |
| handler leader query | handler `+0x0` | native query pointer |
| handler civilization query | handler `+0x8` | native query pointer |

Native scalar/vector size getters are available (`WasEverAlive` `0x2103ff0`,
`GetHandicap` `0x1fcc600`, vector `size` `0x1fe5ad0`, definition `GetCount`
`0x2114220`). **Do not call `GetAllowedOperationTypes` (`0x21143c0`) for readonly
capture**: it tails `FAutoVariable.edit` and may dirty the archive. Copy typed
fields directly from valid live objects; do not deserialize pointers or capacities
as portable state. Check readable backing extents and bounded length arithmetic.

For a Read-side snapshot the exact immediate return site after the real
`InitializeLists` call is `0x2145669`, before any following Military saved fields
are read. Attribute the same owner and call occurrence, prove normal completion,
and copy while objects are paused before any mutation, uninitialization, rule
reload or destruction. Capture prior state at the corresponding call boundary
if the initialization-input route needs it. A Save-side snapshot at the writer's
actual guard describes that writer occurrence; it does not automatically
establish the Read-side initialized state for a later load.

The finite remaining dependency is one genuine, attributed initialized input/result
capture at those boundaries with the stated identities and complete vectors.
This audit does not supply such a capture or authorize changing its native profile,
forcing flags, synthesizing objects, patching branches, executing RNG/policy or
masking unknown bytes. It also establishes no planning-only exclusion.

Reproduce the static checks:

```sh
python3 tools/linux/ai_military_initialization_audit.py \
  --output target/linux/ai-military-initialization.json
```

