# Bounded Linux reporting effects observation

This tooling prepares one passive replay of the accepted corrected conditional SkipTurn profile. **No native capture is claimed by this checkpoint.** Its purpose is to observe the actual Unit readiness branch and compare the active YieldMonitor's live cache contents, Trade membership/order and PlotYields counter before and after its original callback. Complete Rust gameplay state/actions/turns remain unimplemented.

The [prepared metadata](../evidence/linux/reporting-effects-observer.json) binds the exact Linux XP2/host, archived preload `907b006e...`, configuration `b6ca58d5...`, archived source identities and the observer source. Preparation independently decodes the accepted before-Core Trade graph with the production codec and freezes all 182 active edge/route identities, endpoints and forward paths in a **private** configuration file. It does not execute original code or select/fabricate an actor. The actual readiness actor is compared with the selected identity from the frozen parent runtime.

The final plan uses at most **three hardware breakpoints**:

| Site | Capture and interpretation |
|---|---|
| XP2 `0x2c28137`, `84c07518` | Actual HasAdjacentMove return AL inside CanCyclePast; RBX is the genuine Unit. Record identity, owner, location, activity, NeedsAttention and actual 200-byte operation queue/count. Require actual frame PCs in the original CanCyclePast, IsReadyToSelect and FillVariants bodies; retain the backtrace too. A true return takes the original branch that skips both HoldCycling fallbacks. |
| XP2 `0x2c66430`, `534889fb` | Original YieldMonitor::OnMessagesProcessed entry. Armed phase 5 opens one interval and captures the actual receiver, cache headers/backing values, native Trade snapshot and actual PlotYields system. |
| Original caller return, expected XP2 `0x272361e` | Installed only during that interval. A matching thread **and RSP equal to entry RSP plus eight** identify the actual callback return; other hits at this shared dispatcher PC are counted separately. Capture the same surfaces again, compare rows and remove this breakpoint. |

The readiness point remains enabled during the earlier Unit cache callbacks, before monitor entry. It reports every armed phase-5 hit up to 32, then explicitly records disabled coverage. It does not infer actor/queue state from matching event counts. An actual old-empty AWAKE row requires the observed parent identity, AWAKE activity, native queue count zero and the required caller chain. AL is retained as an unsigned byte alongside its boolean interpretation. A false result leaves the original HoldCycling query chain in scope; this observer does not substitute a result.

The earlier hypothesis that ordinary monitor GetYield calls might enter PlotYields::Add/Remove was corrected before native execution. Original GetYield supplies a null contribution list; GetYieldFrom preserves/forwards it, and GetPlayerYieldFrom's null branch skips BonusYieldFromModifiers/Add/Remove. The observer independently gates that exact byte chain. Consequently **Add/Remove breakpoints are not part of this final plan**, and no runtime absence claim for those methods is made. General nonnull-contribution calls still have persistent counters, object lifecycle signals and other exact effects.

The actual PlotYields receiver is read without calls through current Game → pointer `+0x268` GameEffects → pointer `+0xa8` PlotYields. Its raw 88 bytes, counter `+8`, ID vector and shared-object vector bytes are captured at monitor entry/return. The pointer chain is read again at return; a changed system identity is reported without rereading the old address or attributing a counter delta across different objects. A raw wrapping counter delta is reported as an **unattributed live difference**, never as a count of unseen Add calls or a complete authoritative-state classification. Complete counter-writer coverage is not established by the known constructor/Add writers.

Monitor collection reads use the verified schema: std vectors of 32-byte plot IndexLists and 80-byte TradeYields records; IndexList data pointer `+0`, allocated element count `+8`, current count `+16`, plus its complete raw header. Trade cache keys are actual origin/destination ComponentIDs, followed by origin/destination yield lists at `+16/+48`. Current values and raw headers/pointers are retained. The known fixture has 6,996 plot entries and 182 trade entries, but counts alone never certify membership. Bounds are 10,000 plot entries, 1,024 trade entries and 128 values per IndexList. Invalid/out-of-profile pointers or sizes produce an explicit observation error; no inferred data fills the gap.

Native Trade snapshotting follows the original current-Game component lookup and its 288-byte component-entry stride, then the actual Trade ID-map. Occupied hash slots are visited in increasing native slot order, skipping hash `0xffffffff`; the actual live active flag selects routes. For each active edge the observer captures its pointer, edge/route IDs, endpoints, complete raw 536 bytes and ordered forward path. It compares those values with the frozen production-decoded parent records and compares the **multiset of live route endpoints** with the actual monitor cache keys. This establishes the observed identities/order/path preconditions rather than assuming 182 matching entries imply the same routes. The separate origin/destination lookup-index consistency is not observed; it remains explicitly unproven.

Numbered before/after snapshot JSON files preserve sequential intervals without overwriting prior artifacts. They preserve raw pointers, ASLR differences, list headers and value bytes. Row comparisons retain all raw differences. The counter and cache observations do not cover complete live state/RNG or permit excluding AI/cache objects by namespace. Nested monitor entry, a wrong native caller, a byte mismatch or a bounded-read failure stops the observer with an explicit error. An open interval at process exit, missing hardware-install proof or a missing matching return cannot establish any scoped absence or valid before/after comparison.

Prepare and validate offline:

```sh
python3 tools/linux/reporting_effects_observe.py --self-test \
  --output target/linux/reporting-effects-observer.json \
  --config-out /tmp/reporting-effects-config.json \
  --raw-output /path/to/new/private/replay-directory
```

The native lease owner creates a fresh private replay directory, freezes the tool/configuration hashes, and uses the existing accepted driver with its archived preload, source, initializers, command and five Save paths. This tool supplies no launcher and creates no additional game Saves. Load it from the driver's GDB Python block after `starti`, using the frozen source and config:

```python
path = '/path/to/frozen/reporting_effects_observe.py'
namespace = {'__file__': path, '__name__': 'c6_effects_observer',
             'C6_EFFECTS_CONFIG_PATH': '/tmp/reporting-effects-config.json'}
exec(compile(open(path).read(), path, 'exec'), namespace)
```

Ordinary GDB loader instrumentation must be documented separately. There are no inferior calls, field writes, software game breakpoints, native helper edits or policy/flag bypasses. The existing original helper deadline stays unchanged. Timing failures remain failed isolated diagnostics; the observer cannot extend the deadline or justify another replay silently. Acceptance of the observation requires the usual original balanced command/worker/cleanup and all five Save/command comparisons, plus installed byte-gated hardware points and a closed matching monitor interval. Full-host admission and gameplay parity remain false; raw Save→Load **89,198-byte failure** remains unclassified and unmasked.

All private configuration, snapshots and raw observations stay local outside the pending 271-file upload approval. This checkpoint changes only tooling/documentation/public hash metadata. Native execution must await the designated lease owner's scheduling and independent review.
