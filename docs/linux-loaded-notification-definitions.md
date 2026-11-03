# Active notification definitions for loaded upkeep

`loaded_actions::notifications::Definitions` builds the actual ordered definition
input for post-Add notification upkeep from the admitted activated rules database.
It preserves all fourteen columns read by Linux
`NotificationCollectionBuilder::BuildDefinition` (`0x2204bb0`), using its exact
`ORDER BY rowid` query (`GetBuilderSql`, `0x2204e70`). Database row IDs stay
separate from collection indices. Native type hashing and collection lookup
precedence reuse the existing Linux registry implementation.

Admission requires the registered activated rules profile and its pinned report,
then compares actual static resources with the loaded snapshot's admitted
resources. An observer-profile hash is not substituted for rules identity. The
projection preserves nullable text, rejects unsupported text/conversion inputs,
and reads the six Boolean columns as the proven 0/1 values of this resource
profile. It does not silently generalize other SQLite conversion behavior.

The actual collection contains 191 definitions. Its auto-notify entries, in order,
are ChooseTech, CommandUnits, CityRangeAttack, ConsiderGovernmentChange,
FillCivicSlot and ChooseCivic. This is an activated database projection; the
six base XML discovery candidates alone did not establish it. `SendAutoNotify`
checks each definition's auto flag before reading its type hash. Consumers must
preserve that order and short circuit, including the retained array end while
synchronous effects run.

Reproduce the source and actual-field comparison with:

```sh
python3 tools/linux/loaded_notification_definitions_check.py \
  --output target/linux/loaded-notification-definitions.json
```

The check pins the original Linux module and complete builder/getter bytes,
compares the native SQL literal, runs the production adapter on the actual strict
loaded World with admitted resources, and independently compares every projected
field, index and type hash with the original query on the activated database.
Complete source preimages and the candidate executable remain in the retained
fixture directory. This is a Rust/source/database comparison; it does not execute
original code or compare native definition objects.

The immutable definitions do not supply mutable notification authority. Actual
creator registration, pool order, locks, default blocking and callback effects
remain required. In particular, native Manager.Create (`0x2d2f700`) falls back to
base Instance.Create when the registered creator is missing/null **or returns
null**. A missing creator input cannot become a successful no-op, and null
subtype creation cannot skip the real fallback. The concrete ChooseTech query
and creation implementation is tracked separately. Full notification Update,
SkipTurn, affected state, gameplay RNG and turns remain unaccepted.

The source-backed `base_auto_duplicate` leaf covers inherited duplicate slots
with a null-parameter auto candidate. It compares type, location and target
ComponentID, then requires the existing discriminator key `55c7a3f8` to be absent.
It does not compare the candidate's own recycled ID. IsNeeded and actual subtype
binding must be resolved before invoking this leaf; it is not a replacement for
whole IsRedundant or the CommandUnits/CityRangeAttack overrides.

The [initialization audit](../tools/linux/loaded_notification_initialization_audit.py)
checks all six actual auto-subtype `+0x80` slots: these call base InitializeText,
not reporting Send. Add assigns the new ID and appends before its real added-event
report. This limits the required logical pool inputs without treating native
addresses/capacity as gameplay state. Constructor/subtype reset and all other
pre-Add reads still need their actual semantics; the check does not claim native
creation or full Update comparison.

`install_attributed_snapshot` admits the saved notification owner into the same
LoadedState once, before advancement. It checks the complete ff31 snapshot and
attributed owner trace, the Manager version/count header `(1, 64)`, all 64
consecutive bounded collection frames, and the full 4,709-byte Manager span.
Every collection uses the existing exact codec. Slot 0 contains twelve records
and next ID 49; the remaining 63 collections are empty. All slots remain present
regardless of membership in the 43-Player loaded roster. Duplicate ComponentIDs
stay separate occurrences, and numeric-ID lookup retains the first list match.

This installation preserves saved fields and their source provenance. It does
not supply omitted locks, default blocking, subtype bindings, creator authority
or a native Read/live-runtime equivalence claim. The initial owner API also lacks
structural append/removal; full notification Update requires those mutations and
stable logical handles across synchronous effects. No extra independently
mutable notification copy is used as an action authority.

The all-slot owner fixture is checked separately with:

```sh
python3 tools/linux/loaded_notification_owner_check.py \
  --output target/linux/loaded-notification-owner-check.json
```

It rechecks every saved notification field after the diagnostic action's real
Unit activity assignment, then verifies that a snapshot replay after advancement
fails without changing revision. These are Rust ownership/integration checks;
they do not replace native gameplay comparison. Each retained run uses a unique
binary name so a concurrent run cannot replace its executable before retention.

The actual twelve saved numeric notification IDs are unique. The retained actual
fixture checks lookup for each of those IDs. Duplicate-ID precedence is covered
by the core owner's separate source-only tests; no native duplicate-ID boundary
comparison is claimed.

The concrete owner queries use the saved/current field mappings checked in
`loaded-notification-owner-queries-source.json`: blocking is header scalar 14
(native Player `+0x7a8`), turn completion is the fifth serialized processor bool
array, and previous activation checks the first processor bool array before
requiring actual current turn-active controls. The EndTurnBlockingTypes stream
operators copy raw signed i32 values; they perform no definition remapping.

`set_owner_blocking` delegates to the same core `set_owner_blocking_with` control
flow as the legacy aftermath provider. It reads the current field, stores a
changed value, reads the retained owner's ID and current local player, then
calls the required global reporting backend for `55f15a7b` only for the local
owner. The callback receives the same mutable LoadedState, allowing supported
broader effects without an actor-only context. Errors retain the preceding store
and callback effects. No reporting collector or missing-consumer success default
is supplied.

The frozen blocking fixture separately exercises this setter after its action
prefix has already failed: the required global backend errors after seeing the
new stored value; the field remains changed. Repeating that value skips both
reporting and revision change. These are isolated source-only integration cases,
not appended action execution or a native setter comparison. The real action
prefix still terminates at its genuinely unavailable reporting backend.
