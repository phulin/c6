# Linux SkipTurn handler result checker

The separate checker `tools/linux/skip_turn_handler_results.py` is prepared for the new `placement_handler_metadata` profile. **No terminal-cleaned successor capture has been admitted yet, and no native handler equality is claimed.** Its admission constant currently rejects execution. The historical count profile `90221282…`, its reports, immutable candidate and candidate `native_compared:false` fields remain unchanged.

The checker reuses the audited genuine input projection and exact candidate/source gate from `placement_count_reconstruction_results.py`: frozen candidate SHA-256 `58171fb1398b148cfc1e94fc882b79723a9571d193cd6d9c375c3c51fb68f59a`, source commit `301f9e9d188f6a5ac815914218d0380f51b25938`, historical manifest SHA-256 `e2925853cea2eaee831e1c2ba57421957a02d659e9822b750c28c2ffee1cc6c4`. The new manifest and exact prepared/runtime/profile identities require a separate closed-capture admission. Archived native sources, runner and preload must agree with that profile.

Only attributed original Map/Unit/Formation wire, actual native player mapping and roster, raw definitions, war relations and plot limit become candidate inputs. The 43 saved Player47 alive flags must equal actual live Core values. The new operation-identity fact is checked independently against every expected row and never becomes a candidate input. Both separate native answer files are first opened after the immutable Rust candidate process has completed. They cannot supply a missing input or replace an unsupported result.

The expected capture retains all 17,584 count rows and 1,262 Unit binding rows, including their whole-stream ordering, directions, membership identities, nullable Plot distinctions and false native count argument. Every actual Unit is queried: 1,241 placed Units each have current and NULL Plot variants, both visibility values and two repeats; 21 unplaced delayed-death Units each have NULL Plot, both visibility values and two repeats. This is 10,012 real original calls, with four separate NULL-Unit wrapper controls that invoke no original leaf and count toward neither native true nor native false. A missing Formation is unsupported error 244, never an inferred false. The frozen preflight predicts 1,214 true and 48 false per Unit, but native answers are compared individually; those predicted totals are not used as answers.

Six complete 80,657,462-byte Core controls must all match SHA-256 `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884`, without masking any span. The 32-byte Game RNG and 32-byte Map RNG controls must agree, with exact lengths. These are known streams, not a complete gameplay RNG inventory. Successful original return, framework/client cleanup proofs and the final uninitialize footer are mandatory before candidate execution or report acceptance. Retained action/policy trace bytes must match the closed runtime identities and contain only proven observer bindings and a zero-action terminal record; policy/action events reject.

Run protocol rejection tests without any original game execution:

```sh
python3 -m unittest discover -s tools/linux -p test_skip_turn_handler_results.py -v
```

The tests use synthetic protocol rows only, never invented candidate inputs or gameplay parity fixtures. They exercise a complete placed/unplaced matrix and rejection of missing, duplicated or reordered rows, wrong identities/directions/plots/visibility/repeats/operation ID/hash, boolean/integer confusion, missing Formation, unsupported handler rows and NULL-Unit controls misreported as native answers. Answer differences are retained as mismatches rather than accepted equality.

Once an independently reviewed capture is terminal-cleaned and the admission is frozen, the offline invocation is:

```sh
python3 tools/linux/skip_turn_handler_results.py \
  --capture /path/to/closed/capture \
  --diagnostics /path/to/new/diagnostics \
  --output evidence/linux/skip-turn-handler-results.json
```

This boundary covers the registered handler predicate only. Full Manager legality, action execution, scripts, callbacks, complete gameplay state, all gameplay RNG and turns remain unimplemented or unvalidated. The recorded raw Save→Load 89,198-byte failure remains preserved and unmasked.
