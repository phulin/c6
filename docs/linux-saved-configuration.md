# Linux saved configuration restoration

The supplied Linux host's original `GameCore::Serializer::Manager::LoadConfiguration(Serialization::IStream&)` restores the pinned XP2 benchmark configuration through its genuine singleton providers. This is a native configuration prerequisite, not a complete save load, initialized gameplay fixture, or actions/turn parity claim. It uses no Windows layouts, addresses or providers.

Run the original hash-gated fixture:

```sh
python3 tools/linux/runtime_probe.py --localization \
  --saved-configuration ../c6-assets-linux/Base/assets/benchmark/saves/xp2benchmark.civ6save \
  --saved-configuration-output ../c6-ghidra/evidence/linux-configuration/game-write-new.bin \
  --output target/linux/saved-configuration.json
python3 tools/linux/saved_configuration_compare.py \
  --save ../c6-assets-linux/Base/assets/benchmark/saves/xp2benchmark.civ6save \
  --native ../c6-ghidra/evidence/linux-configuration/game-write-new.bin \
  --output target/linux/saved-configuration-comparison.json
```

The runner pins the complete Linux benchmark SHA-256 `a328d643201cb11ba226eee46c782f85b0df36b2a7a4e1a32372da8abe9cddb8`, original host, GameCore, providers and adapter source identity before execution. Native bytes stay outside the public repository. Existing output files are refused. The adapter accepts only the independently observed first 103,901 bytes and seeks the genuine native MemoryIStream to 8,842, bounding the operation before compressed gameplay bytes.

The original Linux function at `0x324f4a0` returns result zero and stream offset 103,901. It reads configuration version 3, metadata version 1 and metadata, restores Game configuration with its original extraction operator, loads User configuration and player settings into the native pending objects, and reads the last-starting configuration. The original host's `User::Instance::LoadToPending` at `0x31fe610` independently confirms version 1, map, player count, signed player ID and map framing. The pinned fixture has one user-settings entry for player 0. This framing is validated for these bytes; malformed/general save behavior is unproven.

After releasing both native input stream and its backing bytes, genuine getters still report `RULESET_EXPANSION_2`, one enabled and one required mod, both ID `4873eb62-8ccc-4574-b784-dda455e74e68`, and player 0 `CIVILIZATION_PHOENICIA` / `LEADER_DIDO` / team 0. These queries test retained provider validity at that point; they do not establish safe module teardown or that the corresponding mod's rules content has been configured.

The original Game::Write output is 57,043 bytes; saved Game configuration is 57,023 bytes. Independent Python structural comparison reports one added value and three flag differences in [saved-configuration-comparison.json](../evidence/linux/saved-configuration-comparison.json). The original loader intentionally preserves the caller-selected GameMode. Matching Linux DWARF (`GameModeTypes`, DIE `0x2c8ce`) identifies saved value 3,915,931,367 as SINGLEPLAYER; the adapter selects this mode through the genuine setter before restoration. The original restoration adds game key `24084b20` with unsigned value 4,294,967,295, which is absent from the saved map. The precise native substep introducing this key remains untraced. Therefore this diagnostic does not claim byte-identical or value-identical restoration. Flag differences are retained explicitly, not silently discarded by the comparator.

User settings remain in the original pending state chosen by LoadConfiguration. The adapter does not guess when to call ApplyPending, select an invented local player, publish load completion, configure modding content, or alter serializer request flags. Restoring pending configuration is not proof that it is active. Successful full native Load remains a separate required checkpoint.

Private Linux disassembly evidence is in `../c6-ghidra/evidence/linux-configuration/load-configuration.txt` and `user-pending.txt`; `game-mode.dwarf` records the matching enum evidence. Saved application-version metadata remains separate from the selected current Linux engine identity.

The native maps have a separate owner lifetime from the input bytes. The SHA-gated [ownership audit](../evidence/linux/saved-configuration-ownership-contracts.json) verifies that primitive-array deserialization allocates payload storage and uses the original MemoryIStream raw reader with copying enabled. `TypedVariantMap::Copy` inherits the source map's manager. The host Game configuration owns that manager at object offset 8; a copied core configuration must be destroyed before that owner.

A passive GDB observation of the genuine framework's Base context proves a teardown use after free, summarized in [saved-configuration-manager-lifetime.json](../evidence/linux/saved-configuration-manager-lifetime.json). Original `LoadConfiguration` returns zero at offset 103,901. With later diagnostic getters and writeback skipped, framework destruction deletes the host configuration owner, then unloads the core. The core's configuration destructor subsequently dispatches through the exact deleted manager address. The captured child receives SIGSEGV; the debugger's zero exit status is not a successful cleanup. This isolates a configuration-owner dependency, independently of network attachment and output serialization. It does not justify retaining input bytes or omitting cleanup. A corrected original release sequence still requires native validation.

Reproduce the bounded static and passive-trace checks with:

```sh
python3 tools/linux/saved_configuration_ownership_audit.py \
  --output target/linux/configuration-ownership.json
python3 tools/linux/saved_configuration_lifetime_evidence.py \
  --trace ../c6-ghidra/evidence/linux-configuration/manager-first-free-v3.txt \
  --report ../c6-ghidra/evidence/linux-configuration/manager-first-free-v3-report.json \
  --output target/linux/configuration-manager-lifetime.json
```
