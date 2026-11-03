# Genuine registered SkipTurn handler fixture

The [original capture controls](../evidence/linux/placement-handler-native-capture-1.json) passed, and [independent comparison](linux-skip-turn-handler-results.md) accepted all 10,012 handler, 17,584 count and 1,262 binding rows. It retains the accepted count producer's complete Unit/Formation traversal,
registry/relations inputs and count calls under a new profile and source identity.
The original count helper, 902 capture, 58171 candidate and reports stay immutable.

The new producer collects every actual stored Unit unconditionally before any
location branch. It does not filter alive, delayed-death, active or on-map status.
In the current fixture all 1,262 Units are included; 21 have no current Plot and
are delayed-death/nonhuman. The handler itself has no corresponding top-level
plot or delayed-death gate. Handler true does not mean a legal Manager action.

Two full independent passes query each Unit with genuine current Plot and NULL
when a current Plot exists, or NULL when absent, and visible=false/true. Current
coverage is 1,241×8 + 21×4 = 10,012 original calls. Four separate NULL-Unit controls
perform no original leaf call and are never handler parity cases. Every actual
query uses the genuine registered handler, bounded runtime ID/hash and a complete
local Operation constructed and destroyed by original code. Manager::Get is a
nonlazy borrowed lookup; no handler/manager construction or teardown is added.
An actual missing Formation returns unsupported before constructor/query rather
than fabricating false. Unplaced Units with real Formations remain included.

Facts, original count expectations and original handler expectations are three
separate exclusive-create files. A non-answer operation_identity fact comes from
the first NULL-Unit registered-handler lookup, before any CanStart; independent
checks bind all expected IDs to it. Neither count/handler answers nor this binding
fact enter the strict candidate input JSON. Native parameter variants and repeat
rows remain explicit and cannot be collapsed or dropped.

The new manifest references original 301f/58171/e292 artifact bytes without a
rebuild. Every historical field remains identical except the successor manifest
schema. New fields describe only the outer query protocol/preflight/admission;
the candidate's internal native_compared:false qualification is unchanged. A new
external attestation must prove native equality before any acceptance. Full
Manager::CanStart, scripts, actions, requests, aftermath, turns and complete
state/RNG remain separate. Human-invalid-location behavior is not covered by this
fixture; NULL Plot arguments for placed humans do not provide that state.

`--placement-handler-metadata` is mutually exclusive with existing query/action
profiles and requires a frozen `--placement-handler-candidate-manifest`. Fresh
private artifacts use these options:

- `--placement-handler-metadata-output` (facts)
- `--placement-handler-metadata-expected-output` (counts/bindings)
- `--placement-handler-metadata-handler-expected-output` (handler rows)
- `--placement-handler-metadata-before`, `--placement-handler-metadata-control-save`,
  `--placement-handler-metadata-after`, `--placement-handler-metadata-repeat`

Outer save/repeat, policy trace, four internal full saves, two known complete RNG
objects and measured cleanup surround both count and handler phases. They are
bounded controls, not all affected live state or getter purity. The separate
admission guard has its own immutable binding cache and original mutable
quiescence/facade lock checks. Unknown work exits before later reads or teardown;
admission clears before original Uninitialize. Source projections preserve every
previous loader/lifecycle profile.

Offline validation before exact integrated admission and any original run:

```sh
python3 tools/linux/placement_handler_metadata_audit.py --output /tmp/handler-producer.json
cmp /tmp/handler-producer.json evidence/linux/placement-handler-metadata-preparation.json
python3 tools/linux/placement_handler_metadata_fixture_audit.py
python3 tools/linux/oracle_admission_cache_audit.py
cc -std=c11 -Wall -Wextra -Werror -fanalyzer -fsyntax-only native/linux/placement_handler_metadata.c native/linux/placement_handler_metadata_fixture.c native/linux/skip_turn_handler_query.c native/linux/action_lifecycle.c native/linux/game_load.c
```

`--prepare-only` emits exact source/preload/configuration identities without
original execution. Runtime uses `lower_core_placement_handler_metadata`,
`placement_handler_metadata_output`, `placement_handler_count_expected_output`,
`placement_handler_expected_output`, `placement_handler_metadata_saves` and
`execution_paths.placement_handler_metadata`. Raw data stays local outside the
pending private upload scope. No saved Begin, UI or network admission is added.
