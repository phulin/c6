# Isolated workspace validation at dd19808

Exact committed source `dd19808b9941da4ad0d75a0142721f165378ad34` passed all three checks from an isolated
`git archive` under `/tmp`; concurrent WIP and later HEAD changes were excluded.

| Check | Terminal code |
| --- | ---: |
| `cargo test --workspace --offline --locked` | 0 |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | 0 |
| `bash tools/check/c_abi_smoke.sh` with offline/locked Cargo wrapper | 0 |

The workspace reports **481 passed, zero failed and 12 ignored tests**. Ignored
private/native fixtures are not passing comparisons. This validates the compiled
Rust workspace and C interface at this exact commit, including the Plot5 decoder
and preceding committed Rust changes. Python/native source audits remain
separately attributed; this batch did not run them or execute original game code.
Complete gameplay state, actions, turns and gameplay RNG parity remain unfinished.

The [report](../evidence/linux/public-source-validation-dd19808.json) pins the
source tree, all 1,768 unchanged committed preimages, source archive,
lockfile, 46 registry dependency identities,
299 external depfile source identities, compiler
executables, logs, 24 test executables and the C ABI
shared library. The complete source archive and manifests, dependency identities,
runner/finalizer, logs and copied executables are retained at
`target/linux/public-source-validation/dd19808b9941da4ad0d75a0142721f165378ad34/`.
Source archive SHA-256:
`6892f9f83e52f252dd46c919bb561720e1e3e282edb97915062e6ac98804f040`.
The isolated extracted source and build outputs remain at
`/tmp/c6-public-source-validation/dd19808b9941da4ad0d75a0142721f165378ad34/`.

This validator made no push. Successors require their own checks or an explicit
unchanged-input comparison; this report does not cover moving HEAD.
