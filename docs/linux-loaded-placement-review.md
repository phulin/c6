# Independent loaded-World placement review

The frozen `70ff4a1868a4b43c66d7a7aa6e261fbc3517709c` candidate passes the
bounded placement and binding comparison from the actual consuming World:
all **17,584 placement rows and 1,262 Unit definition/Formation bindings** match
the pinned original Linux corpus in exact order. No original code was executed
by this review, and no candidate rebuild was needed.

The [independent checker](../evidence/linux/loaded-placement-review.py) verifies
the frozen executable SHA-256
`e24e0a65efa1340179934234f25069c93be524bf71f52e52620ed9706c2fbdee`,
the complete archive against `git archive` of the committed tree, and all 1,553
archived file hashes against its manifest. The archive SHA-256 is
`55938bb8b89aa908fa6f5abdfcdcae1b92f6c61aeb247948fe3e47c18b725fe5`.
The author's comparison checker is independently extracted from that archive
and hash checked, so active shared-checkout changes do not enter the trial.

## Attribution and computation

The strict capture loader admits the original Native3 artifacts, selects the
single genuine `ReadInputCore` occurrence, and consumes its bytes into
`LoadedState`. Its Core SHA-256 remains `ff31f4c5…`; this establishes the
decoding/framing role. The immutable definitions require the actual compatible
resource and configuration identities. Mutable controls and owned war state
come from separately admitted placement/Manager captures, explicitly labeled
`SavedOutputCore` associated-snapshot facts. Full bytes of six saved snapshot
images independently hash to this same Core. This association does not assert
that omitted native Read effects were reproduced or that Native3's Read
process had those runtime controls.

The reviewer independently checked the actual target module and host hashes,
the placement facts `b4922e03…` and Manager facts `3fd5b6e6…`, and the emitted
observed-snapshot provenance. The native expected corpus remains
`bc65088165f1b57473738f6ec5f784c86b838060d3c8f2c87a8bebcd5240c882`.
Its original capture/owner attribution was already independently accepted in
[the native results review](../evidence/linux/placement-count-results-review.json);
this review reuses that unchanged corpus rather than producing a new oracle.

The candidate traverses the imported World's native-ordered Player records,
Units and Formations. Placement queries borrow its actual Map/layers/candidates
and typed records. They do not reconstruct actor records from native getters.
Each of the 1,262 Units and 1,250 Formations receives exactly seven ordered
queries, directions `-1,0,1,2,3,4,5`. The first is the current plot, followed by
the six neighboring directions. Unit binding rows retain owner, ordinal,
generated ID, resolved definition index and resolved Formation ID. Comparison
checks complete row dictionaries; no identity, order, null or count field is
masked. All 225 absent plots remain absent instead of becoming zero-count
plots. The 17,359 present plots yield 13,617 zeros, 3,698 ones and 44 twos.

The World revision remains zero, consistent with the immutable borrowed query
interfaces. This is a read-only placement projection. It does not initialize
every Unit subsystem, execute callbacks, complete a Manager predicate, mutate
an action, advance RNG or execute turns. The candidate's timing covers queries
plus result construction, not complete loading or action latency.

## Independent checks

The reviewer executes the candidate before opening native expected answers.
Exact ordered comparisons pass. The candidate also succeeds with the native
answer file entirely absent, returning the same compared query/binding rows.
Its archived resource adapter and candidate never open that file. The frozen
author's checker runs the candidate and then rejects a corrupted answer file
at the pinned hash gate, preserving the successful candidate output. Corrupting
the actual resource facts instead makes the candidate fail its own closed
artifact hash gate with no accepted output.

Independent comparison checks reject changed count values, changed ordering,
missing coverage, changed Formation binding IDs, swapped Read/snapshot roles,
an invented native-Read control claim, a changed World revision and a promoted
native-Read acceptance flag. These checks concern the concrete frozen corpus;
they do not establish behavior for all possible future states.

[The public result](../evidence/linux/loaded-placement-review.json) records the
source, binary, target, artifact identities, coverage, histograms and negative
checks. Reproduce without building or running original code:

```sh
python3 evidence/linux/loaded-placement-review.py \
  --work-dir /tmp/c6-loaded-placement-independent-new \
  --output /tmp/c6-loaded-placement-independent-new.json
```

Original profiles and results remain unchanged. Complete native Read,
gameplay World/actions/turns and complete RNG parity remain unsupported. The
89,198-byte raw Save→Load difference remains recorded, unclassified and
unmasked; save production is not an acceptance gate for these queries.
