# First action slice: SkipTurn

The library still implements isolated primitives. **No initialized Civ VI gameplay action has passed differential validation.** SkipTurn is the first concrete action target, with a recovered request contract and an executable original-code message-body oracle. Its legality, queue execution, reporting effects and full affected state remain unvalidated.

Target: XP2 FinalRelease SHA-256 `324c51e9ea3531758842e16c69e6cddbefbb226c5b675c3d6e60111646c2e98c`, image base `0x180000000`. Addresses below are RVAs. Names/discovery leads adapt cru121 [Civ6-GameCore-reference](https://github.com/cru121/civ6-gamecore-reference/tree/5af389445118bed2bdebf7a8a42b219c8fba5175), revision `5af389445118bed2bdebf7a8a42b219c8fba5175`, CC BY 4.0. Saved-target decompilation and original instructions establish the observations here, rather than the reference's `verified` label. Raw results remain private in `../c6-ghidra/evidence/actions/`.

## Request and body contract

Register `0x672d90` stores hash **`0x2c681908`** and installs vtable `0xa2b800`. That vtable points to handler CanStart `0x672c40`, Start `0x672e10`, and inherited MakeRequest `0x3ef70`. The manager must resolve the hash to its registered **numeric handler ID**. The wire stores that ID; substituting the hash is incorrect. The convenience function `0x672d40` also resolves this ID and creates a request with secondary `-1`, null shared parameters, flags zero, and two false bytes.

The public path is export RequestUnitOperation `0x5915d0` -> cache manager `0xd1e90` -> operations-manager virtual `+0xf8` / MakeRequest `0x41070` -> handler virtual `+0x70`. The inherited handler request conversion uses generic parameter-bag lookup `+0x60`: `0xc65a8ea6` copies flags to request `+0x18`, and `0x7d835547` copies a secondary integer to `+4`. It only consults `0x97b20a17` and invokes parameter parsing when the handler creates a nonnull parameter object. SkipTurn uses inherited MakeParameters `0x13230`, which returns a null shared pointer. No SkipTurn-specific parameters were found. A minimal driver should pass an actual empty engine parameter bag and record the resulting defaults.

The cache path constructs UnitOperation, calls Init `0x3ab60`, and broadcasts it. Init copies the unit identity, request fields, shared parameter pointer and two bytes, then stores mode zero at object `+0x48`. PutInBufferBody `0x3abd0` writes the following null-parameter payload:

| Wire offset | Width | Meaning | Object offset |
| --- | --- | --- | --- |
| 0 | 2 | Owner bits | `+0x18` |
| 2 | 2 | Object type bits; unit type observed as 1 in Start identity | `+0x1a` |
| 4 | 4 | Unit ID | `+0x1c` |
| 8 | 4 | Registered handler ID | `+0x20` |
| 12 | 4 | Secondary value, default -1 | `+0x24` |
| 16 | 4 | Flags, default 0 | `+0x38` |
| 20 | 4 | Mode, default 0 | `+0x48` |

All fields are little-endian. The parameter object at `+0x28`, when nonnull, inserts its own serialization before mode. The request bytes copied to message `+0x40/+0x41` are **not written by this body serializer**. Their receive defaults depend on construction and must be observed before broader flags are supported. This is a body contract, without message headers, sequence IDs or queue framing.

SetFromBufferBody `0x3ac20` reads the identity and three request words, then calls manager MakeParameters using the decoded ID before optionally reading its payload. Consequently a read-body round trip needs a real initialized operations registry even for null SkipTurn parameters. The current oracle tests writes only.

`evidence/actions/request-schema.json` proposes the narrow future native-driver input: pinned DLL, actual configuration and fixture hashes, owner/unit ID, SKIP_TURN, empty parameters, and a required complete capture. It is a schema proposal, not an implemented action endpoint. Configuration identity must bind rules database content, DLC/mod order and game options; the schema's hash alone does not supply those facts. Missing initialized context, identity mismatch or unsupported action must reject without advancing state. Do not fill fixture/configuration hashes with synthetic stand-ins.

## Legality and mutation dependencies

Manager CanStart `0x3fdf0` performs checks outside the handler: unit status, owner-turn availability, player/session control, interruptibility of an existing queue, disabled operations, optional Lua test-all veto, and a valid plot. Some callee semantic names remain leads. Calling handler CanStart alone bypasses these checks.

Handler CanStart `0x672c40` calls IsHuman `0x67bf0`, which resolves owner player, configuration manager and typed variant data. For a human unit it rejects signed field `+0x4d8 > 0`, activity `+0x288` equal to `0x047c048b` or `0x024a74eb`, and a friendly formation count greater than the unsigned definition value at global `0xb8aa70` target `+0x58c`. The count requires formation component `unit+0xbb8`, coordinates `+0xf8/+0xfc`, GetLocationLayer `0x67990`, map unit-container lookup and formation member unit resolution. For all owners it rejects activity `0x5ff1bbdf`, then requires signed field `+0x920 == -1`. These field meanings are not inferred from offsets alone. The current Windows offsets must not be replaced with Linux layouts.

Handler Start `0x672e10` builds identity `{owner:u16,type:1:u16,id:u32}` using unit `+0x128/+0xb0`; obtains its handler hash through vtable `+8`; and calls reporting `0x5fb890` with RCX containing event `0x3d4006f2` in the low 32 bits and handler hash in the high 32 bits, RDX identity pointer, R8 zero. It then invokes manager SetActivityType via `+0xc0` with `0x047c048b`, and writes completion byte 1 through its fourth argument. It does not directly zero movement points in the inspected body.

SetActivityType `0x41830` compares activity `unit+0x288`, writes through tracked variable `unit+0x278` using `0x2ca0f0`, and emits event `0xe23b112d` when it changes. The write routine can insert the variable into its archive's ordered dirty-variable set before changing the value. A raw activity-field assignment would lose this state. Reporting dispatch is not recovered transitively, so the full event subscriber effects are unknown.

The manager Start `0x41980` can change activity before invoking the handler, reads the active queue node at unit `+0xcd8/+0xce8`, and can update timers based on human visibility. Add `0x3f9c0` clears/interacts with the existing queue, checks insertion, applies defaults and inserts/activates nodes; it also reports automation changes. UnitOperation Execute `0x3aad0` resolves the live unit, calls Add, processes pending diplomacy `0x186e60` and updates end-turn blocking `0x5e410`. These dependencies make synthetic-handler execution insufficient for real action parity.

## Executable evidence and next fixture

Run:

```sh
python3 tools/analysis/actions_message.py --output evidence/actions/message-validation.json
```

The script checks the DLL hash **before mapping/execution**, builds a dedicated adapter with the existing mapper, records source/binary/configuration identities, and invokes unchanged original PutInBufferBody instructions. A synthetic 80-byte message has only its parameter pointer forced null; other bytes, including the nonserialized flags, vary. The callback checks seven write lengths `[2,2,4,4,4,4,4]`, complete 24-byte output and unchanged source object. The corpus covers 125 boundary combinations and 10,000 deterministic randomized cases: **10,125 cases, zero mismatches** against an independent `struct.pack` representation. Nonnull parameter pointers reject before execution. This proves synthetic message-body serialization only. The mapper does not run DllMain or bind imports; its existing RNG logging stub is present but this write path does not use it.

The next required bridge is a genuine initialized unit and registry, obtained from a matching game load/bootstrap. Capture the hash-to-ID resolution, request conversion, body/framing, rejected manager/handler legality results, queue before/after, tracked activity/archive writes, reporting order and subscriber state, end-turn blocking, pending diplomacy, and every RNG stream/count. Include human and nonhuman units, active/inactive owners, existing queues, each rejected activity, the signed `+0x4d8` and `+0x920` boundaries, and friendly count equal to/exceeding its limit. Preserve complete affected-state snapshots and minimized failures. Only after these dependencies execute faithfully can Rust implement the gameplay action and compare full output/state changes.

The current work intentionally adds no Rust SkipTurn implementation: no initialized action fixture or complete affected-state contract exists yet. Geometry/RNG/serialization successes do not close this gap.
