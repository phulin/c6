# Independent full Manager owner and trace review

The bounded owner guards at `55302c3` and FULL lifecycle at `eefe52c` plus
`e262004` meet the reviewed caller's endpoint contracts. No source blocker
remains in this boundary. This is source and mock validation, not admission of
a native full Manager run or proof of action/state/turn equivalence.

The guard checks the retained controller, exported Context, native Context and
Game links, their initialization fields, the cached OS-main-thread witness and
the absence of the separate worker. At the held endpoint it requires the same
nonzero opaque native owner token and depth one; at unlocked endpoints both
native owner and depth must be zero. It rereads roots and lock fields to reject
a change during observation. Native lock tokens are not assumed to be OS TIDs.
The public admission path derives the module base through the actual Context
vptr and obtains the cached witness through the preload's own getter.

These field reads do not acquire native ownership. The genuine facade factory,
single retained reference and original release remain obligations of the
[separately reviewed caller](linux-full-manager-fixture-review.md). Borrowed
Context/Game lifetime depends on that retained framework/controller/module
scope. The guards make no new Context refcount claim and cannot establish
arbitrary concurrent mutation, Lua/GC ancestry or a genuine decision boundary.
They invoke no game methods and contain no actor pointer. Original calls remain
outside the field-only checks, so no stale Unit is dereferenced by these guards
after a potentially mutating predicate.

Queued messages, update/turn-advance flags and positive AI counters are allowed
and preserved. Requiring their absence here would incorrectly reject legitimate
predicate effects. Actual gameplay callbacks, state changes and RNG effects
still require observation and comparison; allowing queued work does not prove
its execution or classify it as planning-only.

The observational trace end retains actual query/report/execute/policy counts,
writes its explicit observational policy, and maps only the old cardinality
error `209` to success. Structural failures `207` and output failures `208`
remain failures. The entire prior trace implementation is an exact source
prefix. FULL alone resolves and requires the new void-signature end callback,
before trace begin or initialization; other selectors keep the unsigned
expected-execution callback and its old behavior. Probe errors remain failures
through cleanup. Existing AI/update/deal hard stops still exit `198` with
`action_acceptance 0`; they are unavailable-run causes, not accepted gameplay
or implementations of external-policy decisions.

The independent driver runs **53 mock cases** against the actual C sources:
30 public admission/ownership cases, nine lifecycle selector/error cases and
14 trace cases. Public admission uses a data-only mock DSO and real
`dladdr`/symbol resolution. Cases challenge missing main witness, another
thread, changed roots, unreadable/overflowing pointers, duplicate and invalid
lock transitions, and a root change between field reads. A protected actor
page and nonempty queued state survive a complete owner cycle. Lifecycle cases
check FULL-only symbol lookup, legacy selectors, missing-symbol refusal before
initialization, and cleanup/error precedence. Trace cases check asymmetric and
maximal counts, `/dev/full`, legacy `209`, and armed/unarmed hard stops.

Reproduce without executing original game code:

```sh
python3 tools/linux/full_manager_owner_review.py --output /tmp/full-manager-owner-review.json
```

The [recorded result](../evidence/linux/full-manager-owner-review.json) pins
stable source and mock executable hashes. Strict C warnings, GCC analyzer,
UBSan without recovery and both producer source audits passed. The producer
audits inspect pinned target bodies but do not execute them. Real mapped-object
identity, main-entry ancestry, genuine lock acquisition, observer/loader
integration, VM/callback coverage and complete affected-state/RNG comparison
remain separate admission and exactness requirements.
