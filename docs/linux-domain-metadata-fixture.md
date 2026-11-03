# Domain metadata fixture admission

This is a separate read-only original-code profile, prepared for native capture.
It has not run original code or established metadata reconstruction parity. Its
exported plot getter answers are expected outputs, never simulator inputs.
Accepted movement helpers, candidates and captures remain immutable.

`--domain-metadata` is mutually exclusive with action, movement and Core-command
profiles. It requires the genuine loaded Core fixture, original main-thread
preparation, cleanup, a fresh private trace directory, five distinct private
artifact paths and a frozen reconstruction candidate manifest. The manifest uses
`c6-linux-domain-reconstruction-candidate-v1`; the runner checks executable bytes,
source archive bytes, Git source hashes and explicit exclusion of expected native
answers from candidate inputs. The exact manifest and digest become configuration
identity before any original execution. A missing candidate prevents preparation.

The loader retains its original Core `RTLD_NOLOAD` reference. The new lifecycle
entry initializes the original controller/cache, opens an independent domain
admission scope, checks quiescence, invokes the exporter, checks quiescence and
identity again, clears admission, then calls original Uninitialize and trace end.
It never requests, queues, pumps or executes an action. Unknown/live state exits
the isolated process before subsequent observations, Save or teardown.

`domain_metadata_fixture.c` owns a separate cache of the same twelve verified
original addresses as the accepted movement admission scope. Mutable controller,
Context and Game state are never cached. Every guard checks original owner and
scope identity, original idle predicates and a genuinely acquired/released Game
facade lock barrier. The domain-only safety refinement exits immediately on a
true Game work predicate, before releasing that facade or reading further state.
The successful idle path keeps the original call order. No facade survives it.

The exporter owns before/control/after/repeat full Core saves and two known
32-byte RNG observations. The runner also captures the two outer full saves,
actual JSONL output, source and preload hashes, rules/DLC/mod configuration,
policy trace and measured teardown. These controls are not complete state/RNG,
action or turn equivalence. Original Save bytes are compared without masking.

Offline checks:

```sh
python3 tools/linux/domain_metadata_fixture_audit.py
python3 tools/linux/oracle_admission_cache_audit.py
python3 tools/linux/domain_metadata_audit.py --output /tmp/domain-contract.json
cmp /tmp/domain-contract.json evidence/linux/domain-metadata-query-contracts.json
cc -std=c11 -Wall -Wextra -Werror -fsyntax-only native/linux/domain_metadata_fixture.c native/linux/game_load.c native/linux/action_lifecycle.c
```

Add the following to the established loaded-Core invocation, using fresh private
paths and the independently reviewed immutable candidate manifest:

```text
--domain-metadata
--domain-metadata-reconstruction-manifest evidence/linux/movement-domain-reconstruction-candidate.json
--domain-metadata-output PRIVATE/metadata.jsonl
--domain-metadata-before PRIVATE/before.bin
--domain-metadata-control-save PRIVATE/control.bin
--domain-metadata-after PRIVATE/after.bin
--domain-metadata-repeat PRIVATE/after-repeat.bin
--action-trace-directory PRIVATE/trace
--prepare-only
```

`--prepare-only` builds and records identity without loading original code. Native
capture still requires the integrated ABI/lifetime review and exact frozen Rust
candidate. Results use `lower_core_domain_metadata`, `domain_metadata_output`,
`domain_metadata_saves` and `execution_paths.domain_metadata`; they do not reuse
movement corpus admission or acceptance labels. Raw evidence remains private and
outside the pending upload scope.
