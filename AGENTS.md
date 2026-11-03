# Working on c6

The objective is an exact headless Civ VI state/action/turn library, checked against original Linux game code. The current Rust implementation covers bounded primitives, wire transport and rules/registry components; complete gameplay state, action execution and turns remain unimplemented. Keep that distinction explicit in code, docs and completion reports.

## Repository layout and migration handoff

- `c6` is the public repository for the final simulator: production Rust libraries, the C ABI/header, production build/interface checks, synthetic regression tests and user-facing documentation. New reverse-engineering tools, source audits, original-engine oracle/harness code, investigative reports and captures belong in private `../c6-re`.
- Private `../c6-re` owns the investigation and validation workspace, including original-engine adapters, differential candidates, source-audit scripts, evidence, capture provenance and private fixture tests. It may depend on an explicitly pinned public `c6` tree; public production builds must not require the private repository or its captures.
- `../c6-ghidra` is reserved for annotated Ghidra databases/projects and the files required to preserve those projects. Non-database investigative tools, reports and captures migrate to `../c6-re`. Preserve existing immutable evidence identities and history during migration; changing a path does not create a new observation or repair an old validation gap.
- Mainline work belongs on `main`. Clean up merged temporary branches only after the coordinator verifies that their approved commits are retained; do not discard unmerged work or rewrite shared history.
- Freeze and hand off coherent committed checkpoints before moving shared files. Record exact public/private pins, file owners, complete retained preimages, capture/source identities and outstanding WIP; preserve untracked and concurrently staged files. Resolve compile-time includes, fixture imports and public/private API boundaries before removing their old paths. Verify public builds without private siblings and attribute private checks to both exact repository pins.
- Root coordinates the freeze and extraction. `reference_reuse` remains the sole public push owner until reassigned. Existing native/oracle work and captures remain frozen during a layout-only handoff; a move does not authorize new original execution. Do not perform bulk moves or branch cleanup as an incidental part of unrelated implementation tasks.

## Task coordination

- Use a separate `gpt-6.1-sol` agent at `medium` reasoning effort for each individual implementation or research task, as requested by the project owner. Give each task explicit file ownership and a concrete result. Use a fresh context when selecting the model/effort.
- All agents share the checkout. Check `git status` before editing or committing; stage only owned files. Every commit must use `git commit --only -m "message" -- explicit/owned/path ...`, even after earlier staging: another agent may stage files concurrently, so a bare commit is insufficient. Preserve other agents' staged and untracked changes. Coordinate shared manifests, protocols and evidence identities; never discard another agent's edits or use broad staging.
- If an owner-approved file is accidentally captured in a shared-index commit, preserve the approved history and coordinate a scoped follow-up. Do not rewrite shared history to repair that inclusion.
- Keep coordination concise. Put raw disassembly, decompiler dumps, captures and long investigative tool output in private `../c6-re`; reserve `../c6-ghidra` for annotated Ghidra project/database artifacts. Send the coordinator only conclusions, relevant addresses, evidence paths, checks and blockers.
- Commit coherent, validated checkpoints and push the authorized project repositories. Coordinate the current public push writer and report commit IDs and unresolved gaps. Ghidra project commits require a clean save/close before snapshotting; follow private `../c6-re/docs/ghidra.md`.
- Designate one private-repository push owner at a time. After committing immutable closed-project snapshot chunks, release snapshot-write ownership so other verified descendants can be prepared; batch those descendants into one push. Do not launch duplicate concurrent uploads of large snapshot packs.

## Exactness and evidence

- The user explicitly defines the simulator as yielding to external policies for **all player decisions**. Execute exact rules/upkeep between decisions and yield at the next genuine decision boundary; native AI policy choices must not silently fulfill actions or end turns.
- Exact gameplay state and gameplay RNG remain in scope. Native AI planning internals are excluded only where a planning-only role is proven. Do not blanket-exclude AI-namespaced objects: gameplay-affecting data, diplomacy, deals, NPC state and RNG remain exact regardless of ownership/name.
- Civ VI save production and binary-identical save rewriting are not success requirements or action-acceptance gates. Native Save output and roundtrips are decoding diagnostics and omit unsaved runtime state; acceptance still requires all affected gameplay state, RNG, ordering and logical identity to match. Keep unknown save bytes unclassified and unmasked, and retain raw Save→Load differences, including the recorded 89,198-byte failure. Proven serialization-only differences do not block live action implementation; unresolved gameplay effects remain validation gaps. A planning-only exclusion must be evidenced rather than inferred from a byte range or class name.
- The user directs Linux-only execution and acceptance for the complete state/action/turn goal. Target supplied `libGameCore_XP2.so` SHA-256 `a0b6611f45ed28fa7bf6e18d608cafddbdd5b1c20cfd880cb50937af731c1faa` with matching Linux host, dependencies and inputs. See `docs/targets.md`.
- Preserve Windows code/evidence as frozen historical work. Do not start Windows-specific tasks, run Wine/Windows native tests, or extend Windows validation unless the user changes this direction.
- Keep exactness and coverage profile-specific. Historical Windows RNG/geometry/message and rules-data parity cannot count as Linux support; validate against original Linux code and export actual Linux rules inputs. Do not reduce the full objective to a lifecycle or primitive subset.
- Linux/Windows layout differences do not establish relative age. Use symbols/DWARF and the pinned reference for discovery, then validate reused contracts against the Linux target's bytes and native fixtures. Public-depot identity, shared names and Steam build numbers do not establish cross-platform behavior equality.
- Never represent approximate mechanics, placeholder actions, opaque bytes or a toy simulator as exact Civ VI behavior. Unsupported operations must be explicit and must not mutate state or count as passing parity tests.
- Gate original-code execution by exact module SHA-256 and configuration identity. Record the rules database, enabled DLC/mods and configuration for any future initialized game fixture. RVAs, old symbols and Linux layouts are evidence leads until verified for the target binary.
- An added operation needs target-version evidence, a reproducible native oracle, boundary fixtures and comparison of outputs plus all affected state. Preserve minimized failing cases. Do not infer complete subsystem or turn equivalence from a primitive test.
- Preserve deterministic ordering, wrapping arithmetic, stream identity and complete RNG state. Do not substitute a host PRNG. Keep logging/service stubs and synthetic object fields documented as limits of the test context.
- Keep simulator state transport distinct from actual Civ VI save loading. Retaining unknown sections does not establish their semantics or permit advancing them correctly.
- Preserve attribution and license boundaries for acquired reference material. Do not commit proprietary DLLs, game assets or private Ghidra project data to this public repository.

## Validation

Run checks appropriate to the changed boundary. The common commands are:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
bash tools/check/c_abi_smoke.sh
```

Use the identity-gated native and differential checks owned by private `../c6-re` when changing supported mechanics or their adapters. The private workspace owns the relocated checks and their complete dependencies; public production checks must run independently of private siblings. Candidate executable hashes change on rebuild; generate a fresh corpus for the tested binary and preserve the configuration gate. An unavailable oracle is a validation gap, not a passing comparison. Benchmark only implemented behavior and report what the measurement excludes.

Reproducible fixture candidates must retain complete source preimages through a reproducible committed tree or a complete immutable source archive, together with dependency and executable identities. If concurrent WIP is genuinely necessary, retain its exact contents and identify its differences from the recorded commit; a hash list alone is insufficient. Preserve limited historical reports honestly rather than relabeling missing or mixed provenance as a coherent committed build.
