# Offline full Manager preload plan extraction

The [extractor](../tools/linux/full_manager_observer_plan.py) reads an exact compiled **owned preload**, extracts actual symbols/body fingerprints and source-backed caller return sites, and retains its binary/source preimages. It never loads the shared object or executes original code. Its output is a plan **fragment**, not an admitted runtime profile or gameplay comparison. Complete state/actions/RNG/turns remain unimplemented.

The initial draft is `target/linux/full-manager-draft/preload-draft.so`, SHA-256 `7c49002e25cda6c9efc3bad78d7c7957f6d2158c8780cecbbd7a1714496ecf70`. Its retained `build.json` records GCC 13.3, source/link order and flags. The final CLI may link differently: regenerate from the actual final preload rather than reuse draft RVAs. The [audit](../evidence/linux/full-manager-observer-plan-source.json) exercises the actual draft and synthetic counterexamples without native loading.

The tool extracts the eight required marker/fixture/invoke/run_bound/notify/profile/main/witness symbols and four enclosing lifecycle functions, with complete function-body hashes. The draft marker is at RVA `0x1f251`, with all eight symbol bytes retained; its optional CET prefix reaches the original return at `+4`. The main-entry OS TID object is eight bytes at RVA `0x31468`. These are the caller's own values, never conversions from the core u32 owner or Lua ASL ThreadHANDLE.

The actual Manager indirect call is at draft RVA `0x1fb81`, returning to `0x1fb84`. Extraction checks the loaded `Bindings.can_start` field at offset `0x38`, its original first-argument RBP source and no intervening slot/register clobber. Other indirect lifecycle edges are tied to their actual dlsym selector string, result slot and call-register reload; overwritten/escaped slots, partial-register clobbers and intervening caller-saved clobbers reject. This is deliberately a conservative O0 contract, not a general disassembler/value-flow engine. Unsupported or ambiguous compiler output must be reviewed rather than guessed.

The fragment contains the exact eight-frame owned caller prefix through `mod_probe_main` and all nine marker-phase prefixes, including entry/CET alternatives. Phase constants and the compiled four-phase observation loop are checked. It provides the adapter fields `caller_suffix_frames`, `marker_parent_frames`, `compiled_symbols`, `live_bodies`, marker address and `main_entry_witness`. Raw mode leaves `window_parent_prefixes={}` and HKS/GC classification unresolved. Additional libc/startup frames can be observed live beyond the verified own-main prefix; predicting them is not a prerequisite for raw copying.

One verified immutable preload byte snapshot supplies ELF parsing, disassembly and retained output, even if the source pathname is replaced during extraction. The manifest is read once. Each declared source preimage must match before it is retained, and copied binary SHA must equal the requested identity. Output directories are exclusive. The exact extractor source, manifest, owned-preload disassembly, binary and source preimages remain under the named target output directory. Only declared source bytes are attributed: the draft build manifest lists 21 C inputs but no header hashes. Final prepared source/header closure and review remain separate from this draft extraction.

Use either the draft manifest or a future unexecuted prepared profile:

```sh
python3 tools/linux/full_manager_observer_plan.py \
  --build-manifest target/linux/full-manager-draft/build.json \
  --output-directory target/linux/full-manager-draft/fresh-plan-fragment
python3 tools/linux/full_manager_observer_plan.py \
  --prepared-report /absolute/path/to/final-prepared.json \
  --output-directory target/linux/full-manager-final/fresh-plan-fragment
python3 tools/linux/full_manager_observer_plan_audit.py \
  --output target/linux/full-manager-observer-plan-source.json
```

The remaining concrete inputs are the actual final CLI-prepared preload/configuration/module/source identities, a reviewed stack/source packet using these regenerated own prefixes, and fresh private capture paths/sole-runner supervision. No static unknown-HKS completeness requirement is added. Callback identities/effects, full VM/GC state, complete gameplay RNG and native action parity remain unaccepted; Save production or byte-identical rewriting is not an action gate.

## Actual CLI-built static assembly

`full_manager_observer_plan_assemble.py` binds an actual `c6-linux-full-manager-preload-built-v1` report to a freshly extracted fragment. It verifies every declared source plus runner, actual preload, original host/Core and declared libc identities, then emits the raw static plan and stack/source packet. The real adapter validator is exercised offline with an explicitly synthetic temporary envelope; that envelope is deleted and is never represented as the normal CLI-prepared native gate.

The first actual CLI-linked preload is SHA-256 `7c6c5b25a593202bf4d03c69d8b66e46830855503c373bf9b8c70b12a586d310`. Its initial 92-source binding and complete owned-prefix bundle are retained under the private native-1 evidence directory and summarized in [the assembly evidence](../evidence/linux/full-manager-final-static-plan-source.json). This initial bundle remains historical while the sole runner fixes its Python supervisor. Driver/runner source changes require a fresh source-map/stack packet and plan hash even when the C/preload/frame identities do not change. No original execution or native admission follows from assembly.
