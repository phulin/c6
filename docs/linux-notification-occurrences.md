# Stable notification occurrences

`player_notifications_owner::Owner` now owns structural list occurrences in
addition to all 64 saved collections. This is bounded storage support for the
actual notification provider, not an implementation of native Add, Release,
Update or complete gameplay state/actions/turns.

`ObjectId(slot,index)` retains its initial decoded values. Its index is now a
stable occurrence serial: removal never renumbers or reuses it. `ordinal_of`
resolves the current list position in constant time. `object(slot,ordinal)` and
indexed `record`/`record_mut` still take current list ordinals. Numeric-ID lookup
still returns the first matching occurrence in list order, including duplicates.
Removing an unrelated occurrence preserves survivor handles and order. Released
handles return `StaleObject`. Handles remain owner-relative and must not be
transferred between owners.

`append_initialized` appends a supplied record and runtime metadata after bounded,
fallible capacity preparation. It does not assign IDs, increment next ID, set
added turn or execute callbacks. `remove` detaches supplied storage, returns its
record and runtime metadata, and retains survivor order and the next-ID counter.
It does not call Release or destroy native payloads. A provider must execute the
required Release before erasure and preserve any preceding effects on failure.
There are no successful no-op factory or callback defaults.

`Runtime` keeps omitted live fields explicitly optional. Decoded records receive
unknown (`None`) values for actual live binding, default blocking, group and
vtable. This cannot be interpreted as constructed-empty initialization. Existing
`notification_lifecycle::LiveNotification` separately identifies a logical live
object and pool binding. Reusing that object after release creates a new list
occurrence; its earlier occurrence stays stale. Exact LIFO pool choice, subtype
initialization and destruction are obligations of the provider.

Diagnostic `OccurrenceLimits` bound active records and lifetime occurrence
history separately. Removed occurrences retain small tombstones so a storage
budget failure rejects rather than reusing a stale handle. Initial history
allocation and append reservations are fallible; initial counts are checked
before metadata allocation. No world clone is needed. Removal updates following
ordinal mappings in linear time, matching the ordered vector storage boundary.

The [static audit](../tools/linux/notification_occurrences_audit.py) gates the
original Linux XP2 SHA and checks Add's order: conditional added turn, wrapping
counter increment and SetID, append, virtual `+0x90`, ID capture, reporting, then
ExpireAllSupersededBy. Cleanup calls virtual Release before erasing each node.
These source checks describe the integration obligations; they do not assert
that the raw storage methods execute those gameplay effects. The
[evidence](../evidence/linux/notification-occurrences.json) records method
fingerprints and exact checked sites. No original code was executed.

```sh
cargo test -p c6-core player_notifications_owner --lib
cargo clippy -p c6-core --lib -- -D warnings
python3 tools/linux/notification_occurrences_audit.py --output evidence/linux/notification-occurrences.json
```

Tests cover duplicate precedence after removal, stable survivors, stale handles,
explicit runtime retention on detach/reappend, independent active/history limits,
and a supplied callback failure retaining its earlier committed storage prefix.
Callback tests are storage integration cases, not native Add comparisons.
Source matching still encodes the exact saved fields without runtime metadata.
The historical 89,198-byte raw Save→Load failure remains unmasked and unresolved;
full runtime/gameplay restoration and native state/RNG acceptance remain open.
