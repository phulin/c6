# Working on c6

The objective is an exact headless Civ VI state/action/turn library, checked against original Linux game code. The current implementation covers isolated primitives. Keep that distinction explicit in code, docs and completion reports.

## Task coordination

- Use a separate `gpt-6.1-sol` agent at `medium` reasoning effort for each individual implementation or research task, as requested by the project owner. Give each task explicit file ownership and a concrete result. Use a fresh context when selecting the model/effort.
- All agents share the checkout. Check `git status` before editing or committing; stage only owned files. Coordinate changes to shared manifests, protocols and evidence identities with their owners. Never discard another agent's edits or use broad staging that captures unfinished work.
- Keep coordination concise. Put raw disassembly, decompiler dumps and long tool output in evidence files, with proprietary project artifacts in `../c6-ghidra`. Send the coordinator only conclusions, relevant addresses, evidence paths, checks and blockers.
- Commit coherent, validated checkpoints and push the authorized project repositories. Report commit IDs and unresolved gaps. Ghidra project commits require a clean save/close before snapshotting; follow `docs/ghidra.md`.

## Exactness and evidence

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

Use the native and differential checks in `README.md` when changing supported mechanics or their adapters. Candidate executable hashes change on rebuild; generate a fresh corpus for the tested binary and preserve the configuration gate. An unavailable oracle is a validation gap, not a passing comparison. Benchmark only implemented behavior and report what the measurement excludes.
