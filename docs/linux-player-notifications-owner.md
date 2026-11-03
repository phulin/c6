# Owned PlayerNotifications storage

`player_notifications_owner::Owner` retains all 64 ordered Linux notification
Manager slots using the existing `PlayerNotifications` and `Notification` types.
This is concrete saved-field ownership, not complete native loading, live
notification state, Manager::Update, actions or turns. Locks, pools/lifetimes,
default blocking and actual subtype/vtable bindings remain unknown.

Slot position, the collection's stored owner and each record's full ComponentID
remain separate identities. None is rewritten or inferred from another.
Duplicate ComponentIDs are preserved. Owner-relative `ObjectId(slot,index)`
distinguishes logical list occurrences; it is not a native address and cannot be
transferred across owners. Indexed mutation checks the complete expected current
ComponentID before exposing a record. Unrelated Unit mutations do not invalidate
these owned records. The numeric-ID finder follows original PlayerNotifications
Find (`0x2d30940`): first matching numeric ID in list order. Manager::Find's
additional validity/type routing is not implemented by that method.

There is no structural add/remove/reordering API. Therefore occurrence indices
remain stable under the supported interface. Full notification creation/cleanup
needs separately maintained stable object identity and source-closed list,
pool and callback effects before those operations can be implemented. The raw
next-ID setter does not implement native Add or ID assignment.

`matches_source` validates supplied typed collections against one attributed
Manager span: exact version 1/count 64 header, all 64 existing codec outputs in
order and no trailing bytes. It uses the established codec only for preimage
comparison; it does not produce or rewrite a game save. Total source bytes and
record count are bounded before any encoded buffer. Item/depth limits remain
per-collection codec limits, without a new aggregate Manager item-budget claim.
The containing LoadedState installer owns pristine, one-time witness admission
and checked revision barriers before mutation.

The [source audit](../evidence/linux/player-notifications-owner-source.json)
gates the pinned Linux module and matching host, fingerprints the relevant
Manager/Player lookup and initialization bodies, and reuses the exact existing
ff31 Core and owner trace. Its 64 contiguous frames are calls 201876..201939.
The Manager span is `[76400871,76405580)`, 4,709 bytes, SHA-256
`68c99c43d65778dea5f1d49a7215d9d925b405f639d413b28e2c4f5d6f601094`.
Slot 0 has 12 saved records and next ID 49; the other 63 slots are explicitly empty.
This attributed component cut is not a general Core index or proof of native
runtime locks/defaults. No original code was executed.

```sh
python3 tools/linux/player_notifications_owner_audit.py --private-output ../c6-ghidra/evidence/linux-player-notifications-owner --output target/linux/player-notifications-owner-source.json
cargo test -p c6-core player_notifications_owner --lib
```

Focused tests cover slot/owner separation, duplicate identities, exact identity
rejection before mutation, independent counters, exact source matching and total
byte/record budget failures. Full live initialization and notification gameplay
acceptance remain separate requirements. Unknown save bytes and the historical
89,198-byte Save→Load failure remain unclassified and unmasked.
