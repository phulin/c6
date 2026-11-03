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
