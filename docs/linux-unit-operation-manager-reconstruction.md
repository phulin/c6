# Frozen Manager raw-state reconstruction protocol

The new candidate reconstructs fresh-Read disabled lists, definition lookup identities, selected phase order/segment flags and saved Player turn-active values. It uses the accepted adapters `7e6421d`/`94e81a1`, independently reviewed in `944a0ad`. Its input/output protocol and projection driver are committed before the new native facts are captured. This is **raw-state reconstruction**, not execution or acceptance of FullManager CanStart, scripts, actions or turns.

The exact target module is Linux XP2 SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa`; host identity is independently required. Attribution and source contracts remain in [the concrete adapters](linux-unit-operation-manager-world.md). This candidate is restricted to the attributed canonical Core `ff31f4c543e073e13080f456eb9698467c06a13509570558fb3ba5b31e89d884` and owner trace `528721084cdabd8b1eb5b1ae8c57b0b9a0a5ab9c2729aa4f1c5074f4b0cba8ae`, with all 65 player slots, 43 resolved Players and 1,262 Units. A different corpus requires a separately reviewed protocol/profile.

## Strict input

The CLI takes one JSON path and writes one JSON result only after successful reconstruction. Any malformed/unsupported required input exits nonzero without partial stdout. Serde rejects unknown and duplicate fields, including fields that would carry native comparison targets. Every file uses a relative path, exact byte length and SHA-256; absolute paths, parent traversal and symlinks escaping the request directory reject.

`c6-linux-unit-operation-manager-input-v1` contains:

- Module, host, configuration/profile, canonical Core and owner-trace identities.
- The attributed complete Map blob and relative directory holding the genuine identity-gated activated diagnostic rules databases.
- `operation_definitions`: only ordered `collection_index` and `hash`. Native FindIndex/HashToIndex observations are forbidden here.
- `registrations`: actual hash/runtime-ID/handler-presence/handler-ID/hash facts. These retain real registration identity; they do not supply lookup answers or handler admission booleans.
- Actual configured phase hash and current-phase integer. Observed runtime phase slots, order, pointers and segment flags are forbidden inputs to independent phase reconstruction.
- Actual raw Game pause byte, automation-processing Player enum and script-system pointer presence. Callback availability/effects are not invented from pointer presence.
- Exactly 65 ordered `player_slots`, containing query ID, raw mapped index and actual stored-player pointer presence. Validity and resolved-pointer results are reconstructed independently, preserving invalid and absent as separate facts.
- Exactly 43 ordered resolved Player records, each with actual configuration slot status, an attributed 98-byte Player47 header blob and complete ordered Unit membership with attributed Unit28 wire blobs.

The candidate validates all Player/Unit owners, IDs, ordinal order and duplicate slots, and verifies every complete Unit re-encodes unchanged. It uses the owned World for retained Player identity/configuration reads. Header decoding follows the exact current 98-byte grammar and rejects noncanonical booleans; flag index 2 is the source-proven turn-active field. No Player header or Unit may be silently omitted, including zero-Unit Players or unplaced/delayed-death Units.

## Targets stay separate

The metadata helper's v2 JSONL contains both raw inputs and comparison targets. The driver projects an explicit whitelist into the strict candidate request. It never passes these targets:

- Native numeric/hash FindIndex and HashToIndex results.
- Current live disabled list size, capacity, default/purgatory byte, raw object storage or value vector.
- Observed selected runtime phase vector/order, nullable phase/segment pointers or segment raw flags.
- Live Player turn-active bytes, native mapping-valid results or native resolved-player results.

Fresh-Read disabled vectors derive from saved Unit hash/value pairs and actual definition order, including proven reset/default/remapping semantics. Selected phases derive from the genuine rules dataset and actual configured phase type, with signed ordering and SQL active-segment resolution. Ties remain explicitly unsupported; observed native order is never used to settle them.

`c6-linux-unit-operation-manager-reconstruction-v1` returns those reconstructed states and complete roster identities. Source-derived tactical permission is explicitly labeled `source_only`; the helper exports no native permission result, so this cannot count as numerical native predicate parity. Raw control fields are echoed as admitted inputs. Script callback support remains unsupported/not inspected, and FullManager support is false.

Only after executing the frozen candidate does the driver compare outputs with native lookup facts, live disabled values/default/size, selected phase identity/order/segment flags and all 43 live turn-active bytes. Capacity and raw allocator/object storage remain retained, unmasked observations that this candidate does not reconstruct. They cannot count as passing complete live-state parity. A mismatch or unsupported phase reconstruction fails comparison; original failing facts and candidate diagnostics remain preserved.

The comparison driver does not establish native lifecycle admission. The sole native runner must separately enforce exact helper/host/configuration/inputs/source identities, full four internal plus two outer Core Save controls, known RNG checks, independent fact repeat, quiescence and original cleanup. Full script registration, callback effects, actual handler interruption and complete action aftermath remain open. No original Manager predicate or callback is invoked by this candidate or driver.

## Immutable build

`freeze` accepts a committed source ID and creates one exact complete `git archive`. All source dependencies and exports come from that archive; it never builds against a mixture of revisions or the shared working tree. It builds the archive's dependencies using locked offline Cargo, compiles protocol tests and builds the release candidate once. The create-only manifest `evidence/linux/unit-operation-manager-candidate.json` records the complete committed source map, archive/binary sizes and SHA-256 values, toolchain/launcher identities, exact build commands and test identity. No native facts or answers are opened during freezing.

```sh
python3 tools/linux/unit_operation_manager_reconstruction.py freeze \
  --source-commit COMMITTED_PROTOCOL_SOURCE \
  --directory ../c6-ghidra/evidence/linux-unit-operation-manager-reconstruction/candidate-1 \
  --output evidence/linux/unit-operation-manager-candidate.json
```

After the separately admitted capture closes, the comparison command requires its actual profile identity and facts, canonical Save/owner trace and genuine rules files:

```sh
python3 tools/linux/unit_operation_manager_reconstruction.py compare \
  --manifest evidence/linux/unit-operation-manager-candidate.json \
  --facts /private/capture/facts.jsonl --core /private/capture/before.bin \
  --trace /private/verified-owner-trace.jsonl.gz --rules /private/genuine-rules \
  --profile-sha256 ACTUAL_PROFILE_SHA256 \
  --directory /private/new-comparison-directory --output /private/comparison.json
```

Protocol tests reject target-bearing/unknown/duplicate fields, unsafe paths and invalid header versions/lengths/booleans. Existing native profiles and frozen handler candidates remain immutable. Complete gameplay state/actions/RNG/turns and the unmasked **89,198-byte** Save→Load failure remain unresolved.
