# Original full Manager predicate observation

The corrected `27ee77e3…` profile completed one original Linux
`Manager::CanStart` call for owner 0, Unit ID 15794187, genuine SkipTurn runtime
ID 36, `visible_only=false`, and a NULL supplied Plot. The original returned
`true`. The fixture did not execute SkipTurn or a turn. This is a native
observation, not a Rust comparison or complete Manager parity.

The natural original Lua path was reached once. The protected call reported
status 0/success 1; the wrapper return bit was false and its output byte was
00. These discriminants do not establish an empty callback registry. The raw
observer deliberately leaves VM/GC parent and Lua owner classification
unresolved and does not decode the heap or registry. Its separate raw
classification is owned by the observer author.

The [control report](../evidence/linux/full-manager-controls.json) verifies
the sealed runtime, 92 source preimages, nine fixture observations and two
caller RNG endpoints. The two known 32-byte RNG objects matched across all
eleven records. Four passive metadata files were complete and identical.
All six optional Save diagnostics were actually byte-equal to the 80,657,462
byte `ff31…` Core. Equality is observational and was not an invocation or
acceptance gate; it cannot prove unchanged Lua state, complete affected state,
or complete RNG coverage.

Actual Controller uninitialization, framework/client cleanup, normal inferior
and debugger exit, and absence of both recorded process generations were
verified. The observer's `original_cleanup_proven=false` is its own limited
scope; cleanup is witnessed separately by the native footers. Unrelated
unreadable `/proc` entries prevent an unrestricted global process-absence
claim. The known trace hooks recorded zero queued/received/executed/report
events; they do not observe every policy or callback.

The untouched actor still had its old MoveTo front. The retained Rust action
prefix first clears that front and inserts SkipTurn, so this is not a paired
activation-seam comparison. Rust's missing script provider and complete
action continuation remain open. Save writing is not a production goal or
action prerequisite.

The earlier `7470…` run remains a preserved observer refusal before the
requested predicate, with no native cleanup claim. Its host-copy mismatch
was repaired through strict actual host/preload/Core/libc path and inode
bindings; the checks were not relaxed.

Reproduce the post-capture control classification without original execution:

```sh
python3 tools/linux/full_manager_control_audit.py \
  --directory ../c6-ghidra/evidence/linux-full-manager-predicate/native-2 \
  --expected-runtime 255603fc41c1768a5da113beeb94b14aa28010da837e71bb9540fd7b16028903 \
  --expected-closed 449c875d1f45d01257ddfc59ded6b1690f10e9a2c76132f9171903a599575822 \
  --output /tmp/full-manager-controls.json
```
