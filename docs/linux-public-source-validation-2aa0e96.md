# Focused committed notification-owner validation

Exact commit `2aa0e9616eca0a348769f6c968c53a6c51273769` was checked from a separate isolated `git archive`,
excluding concurrent WIP. Against the [full dd19808 validation](linux-public-source-validation-dd19808.md),
the only change in Rust/Cargo/include/C ABI inputs is
`crates/c6-core/src/player_notifications_owner.rs`, from `0d78345` and `c4946ad`.
The complete source trees differ in other Python/native tooling, docs and evidence;
those changes receive no acceptance from these Rust checks.

| Fresh check | Terminal code |
| --- | ---: |
| `cargo test -p c6-core notification --offline --locked` | 0 |
| `cargo clippy -p c6-core --all-targets --offline --locked -- -D warnings` | 0 |

All **24 notification tests passed**, with zero failures or ignored tests and
417 filtered out. These include notification-owner storage/traversal and existing
notification consumers. Workspace tests and C ABI smoke were not rerun; their
passing results remain attributed to exact `dd19808`, with the single changed
Core source input explicitly recorded rather than declaring identical libraries.
No broader input difference or focused failure required a full-suite rerun.

The [report](../evidence/linux/public-source-validation-2aa0e96.json) records the
source tree, input comparison, source/dependency/executable identities and logs.
All 1,782 committed source preimages remained unchanged. Complete
source archive, manifests, comparison, runner/finalizer, logs and the test executable
are retained at `target/linux/public-source-validation/2aa0e9616eca0a348769f6c968c53a6c51273769/`.
Source archive SHA-256: `f13417f68db51cd81862015526863e9c617a2a81a2b6e32df21ba60c45fbefc2`.
The isolated source/build outputs remain at `/tmp/c6-public-source-validation/2aa0e9616eca0a348769f6c968c53a6c51273769/`.

No original engine execution, Python/native audit or push was performed.
Complete gameplay state, actions, turns and gameplay RNG parity remain unimplemented
or unvalidated. Later successors require separate checks or an explicit
unchanged-input comparison.
