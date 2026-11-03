# Offline placement differential candidate

The new `c6-linux-placement-count-candidate` consumes exact typed input state and genuine primitive native metadata. It derives current and six directional-neighbor Unit/Formation friendly counts, preserving null positions and native iteration order. Native count results, rivalry answers, layer answers and CanStart answers are rejected as input fields. This is preparation for a new original-code comparison; no native count or handler acceptance is claimed here. Complete state/actions/turns and gameplay RNG remain open, and Engine::apply remains unsupported.

The earlier `23539a3` placement report's candidate binary was built in the shared checkout. Its source hashes matched, but an independent complete committed archive produced a different binary hash. That report retains its bounded qualification and is not rewritten. New candidates are built exclusively from a complete immutable `git archive`, recording source commit/archive hash, all Rust/Cargo source hashes, toolchain details and actual binary hash. Preparation creates new directories and manifests exclusively; old domain/placement profiles remain unchanged.

## Candidate protocol

Run `c6-linux-placement-count-candidate INPUT_JSON EXPECTED_CONFIGURATION_SHA256 EXPECTED_PROFILE_SHA256`. Both independently gated command-line identities must match the input declarations. The supplied XP2 and matching Linux host SHA-256 are fixed in the binary. The runner, separately, must verify native profile, configuration, original serializers/owner traces, purity controls and expected outputs before claiming comparison. No original module executes in this candidate.

Input is one strict JSON object, schema `c6-linux-placement-inputs-v1`, with [the exact machine schema](linux-placement-count-input.schema.json). All listed fields are required; unknown and duplicate fields reject. Wire file paths are relative normal components beneath the input JSON directory and each blob has `{path,bytes,sha256}`. Individual payloads and total input have explicit resource bounds. Input fields are:

- `gamecore_sha256`, `host_sha256`, `configuration_sha256`, `profile_sha256` and `map` (exact current Map wire blob).
- `player_mapping`: the actual 65 original signed mapping values; `player_slot_present`: 65 separately observed actual Player pointer-presence booleans. Mapping validity, known null pointer and missing imported record are distinct.
- `players`: native ascending query-order resolved records, each with `owner`, actual Configuration `slot_status` and `team`, actual Core `alive`, and complete `units`/`formations` memberships. Membership is `{count,complete,entries}`; entries retain `{ordinal,id,slot,actual_owner,wire}`. Every wire identity, owner, explicit slot and current-version roundtrip is checked. Resolved valid non-null players cannot be omitted.
- `unit_definitions`: `{count,complete,entries}` in actual native collection order. Each entry is `{ordinal,index,hash,domain_hash,formation_class_hash,make_trade_route,spy,combat,range}`. Null native definition slots are unsupported by this dense export profile rather than assigned fake hashes. Missing Unit type hashes still reconstruct null binding through the original lookup contract. The runner independently compares captured Find(hash)/Find(ordinal) results; those answers are not candidate inputs.
- `relations`: `{player_count,war_types,complete}` from actual Game.DiplomaticRelations, preserving the complete ordered first-i32 WarTypes vector and its triangular length. Player.Diplomacy is not substituted.
- Actual native signed `plot_unit_limit`.

The existing World owns all decoded records. `Player::from_lookup_slots` accepts verified complete Begin/End/GetCount/Find exports while retaining unknown allocator metadata. It builds one dense lookup index through the largest supplied slot within the native sixteen-bit domain; it invents no capacity/free-list values. `unit_records`/`formation_records` retain native order. Existing Recycled-backed construction and full wire access remain available; allocator-bearing accessors explicitly reject lookup-only input, including a known-empty lookup collection. Mutation remains unsupported as complete gameplay behavior.

Output schema is `c6-linux-placement-count-results-v1`: `count_results`, `unit_binding_results`, `handler_results` and explicit scope/purity flags. Count rows match exporter identity/order with `kind=count_result`, `member_kind`, `owner`, `ordinal`, `id`, `anchor`, `anchor_valid`, `layer`, `direction=-1..5`, `plot_present`; present plots additionally carry `plot`, `bool_argument=false`, and computed `count`. Null plots retain their row and have no count field. Formation layers use the first raw member ID at the Formation's own location, without resolving/death-filtering that member. Full owner/type/generated-ID layer matching differs intentionally from recycled low-sixteen-bit Find.

All actual Unit and Formation memberships are traversed, including orphan Formations, which is broader than just Unit-reachable Formations. Computed Unit definition/Formation bindings are separate outputs for native comparison. Handler results are separately computed from actual player/rules facts; unsupported live inputs remain explicit. This first native profile has no reviewed genuine SkipTurn handler receiver, so every handler result is explicitly native-uncompared. It cannot establish full Manager::CanStart, script vetoes, reporting consumers or action execution.

The offline `player_alive_saved_crosscheck.py` can independently compare actual Core alive fields with flag0 from all43 attributed Player47 headers in the existing exact Core/trace pairs. It parses only the complete98-byte headers and never claims complete Player payload import or configuration/mapping recovery. A report without fresh native metadata explicitly records that the actual-Core comparison remains unavailable; no candidate input or frozen manifest changes.

Before and after queries the candidate checks complete supplied Map/Unit/Formation wire hashes; mutable handler projection is checked against its original Unit. This covers supplied typed records only, without claiming complete native live state/RNG purity. The native runner must independently capture/repeat original count queries and raw Save/RNG controls. The raw 89,198-byte Save→Load inequality remains unmasked.

Freeze committed source with:

```sh
python3 tools/linux/placement_count_candidate.py --source-commit COMMIT --directory target/linux/placement-count-COMMIT --output target/linux/placement-count-COMMIT.json
```

Target methods, binding/source limits and provenance requirements remain in [the placement contract](linux-unit-placement.md). New topology/Formation-layer source gates are in `placement_count_input_contract.py`. No native execution, UI/saved-Begin work, Windows validation or private/main push is authorized by candidate preparation.
