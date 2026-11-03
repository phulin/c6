# Isolated public batch validation at 7937dd1

The committed batch at
`7937dd14c0cb579d245beab16a294058fa02ac03` passed fresh offline, locked workspace
checks in an isolated git archive. WIP and later HEAD changes were excluded.
This is source/build/interface regression validation, not complete Civ VI
state/action/turn implementation or native gameplay/RNG parity.

| Check | Terminal code |
| --- | ---: |
| `cargo test --workspace --offline --locked` | 0 |
| `cargo clippy --workspace --all-targets --offline --locked -- -D warnings` | 0 |
| `bash tools/check/c_abi_smoke.sh` with offline/locked Cargo wrapper | 0 |

There were 473 passing tests, zero failures and 12 ignored tests across the
reported test groups. Ignored private/native fixtures were not counted as
comparisons. The C caller exercised the implemented C ABI, including error
atomicity, bound identity, simulator snapshot transport and numeric streams.
It did not load or execute the original engine or produce a Civ VI save.

The [validation report](../evidence/linux/public-source-validation-7937dd1.json)
records terminal codes, toolchain, log/executable hashes and the exact source
archive/manifest identities. All 1,734 committed preimages were retained and
unchanged after the checks. Full source, archive, manifests, runner/finalizer,
logs and build outputs remain under
`target/linux/public-source-validation/7937dd14c0cb579d245beab16a294058fa02ac03/`.
The archive SHA-256 is
`448cc2797f255b5c3cf6e411dcad903456f2d160237dcb6738df91da79493536`;
the source-manifest SHA-256 is
`9ef1f294b1a8ee227ea61645c65745c87c5d17ed5b50266193a89124eaab38d8`.

The proposed publication successor `5b3667a` changes three paths: independent
review documentation, its evidence JSON, and the new Python checker
`tools/linux/full_manager_closed_review.py`. An explicit comparison finds none
intersects the retained compiled workspace/test/C ABI input manifest, which
includes crate sources, Cargo inputs, generated Rust dependency inputs, headers
and C ABI smoke sources. Therefore the library checks remain applicable to that
successor's unchanged library inputs. **The complete code trees are not identical.**
The Python review's seven refusal cases and exact producer-report reproduction
are separately attributed to its own committed evidence, not rerun or validated
by Cargo/C ABI checks in this batch.

Only `reference_reuse`, the designated push owner, publishes the selected checked
work branch. This validator performed no push or history repair. Later moving
HEAD and unrelated implementation changes are outside this validation unless
independently checked. Raw save differences, unknown runtime state, unsupported
gameplay actions and complete gameplay RNG remain separate requirements.
