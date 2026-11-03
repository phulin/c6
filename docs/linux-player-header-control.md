# Linux Player47 alive header bridge

The pinned Linux Core Player47 header's first of six ordered boolean values is
`GameCore::Player::Instance::m_bIsAlive`. A named `PlayerHeader` accessor can return
`ordered_flags[0]` without changing decoding, encoding or wire order. This closes
one saved field's meaning; complete gameplay state, actions and turns remain
unimplemented.

The target module SHA-256 is
`a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`;
the matching host SHA-256 is
`106d4986567cf60aad787dfac990d089e41775e04b5f40e073f59a3a8ffe74f4`.
Evidence is derived from those original ELF bytes and matching embedded DWARF.
No reference-platform behavior assumption or original execution is required.

The getter `Instance::IsAlive` at `0x2003cb0` reads byte `this+0x260`.
DWARF places `m_bIsAlive`, a 24-byte `FAutoVariable<bool, Instance>`, at
`this+0x250`; that wrapper's `m_value` is at `+0x10`. The writer at
`0x2b4ee00` passes wrapper `this+0x250` to the bool serializer at call
`0x2b4ef2e`, immediately after EndTurnBlocking and before the other five bools.
The reader at `0x2b4f490` passes the same wrapper at call `0x2b4f879`.

The bool operators dispatch through virtual slots `+0x28` (save) and `+0x10`
(load). The relocated vtable at `0x3addb28`, with ABI address point `+16`,
resolves these to `0x2b3e180` and `0x2b3e140`. Both forward wrapper `+0x10`
to the corresponding bool stream primitive. Thus the native reader assigns this
saved boolean to the exact byte read by IsAlive. The audit gates full getter,
writer, reader and wrapper methods, version 47, call sites, vtable relocations
and the matching field layout.

`Instance::GetTeam` at `0x2b31af0` obtains the actual player configuration and
calls its GetTeam. `Instance::IsHuman` at `0x2b31530` obtains global
configuration, selects the embedded player ID's configuration, and calls its
IsHuman; configuration IsHuman at `0x28ec230` tests SlotStatus equal to 3.
Neither has a proven bridge from these fifteen ordered scalar words or six
booleans. They must remain explicit external configuration dependencies.

Reproduce the read-only check from the repository root:

```sh
python3 tools/linux/player_header_control_audit.py \
  --output evidence/linux/player-header-control.json
```

The two existing original canonical Core streams are hash gated and parsed
structurally from byte zero with `core_stream_index.index_core`; only then is
the first Player header parsed field by field. Both first headers are 98 bytes,
contain six canonical boolean bytes, and have saved `is_alive=true`. Fixture
positions in the report are observations, not general parser offset constants.
No trace ownership or callback payload is used. These fixtures do not establish
all 65 slots, all Manager entries, a complete roster, initialized rules identity,
or the correctness of a loaded world.

The accessor describes the saved wire value, and the static reader assignment.
Native later callbacks, load fixups and initialized post-Read state have not been
validated here. A decision provider can use the field only when its imported
state context is otherwise valid; the field alone does not authorize turn
advancement. Unknown bytes remain unmasked and unclassified. The recorded raw
Save-to-Load difference remains 89,198 bytes.
