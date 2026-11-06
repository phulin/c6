Install the tracked pre-commit formatting check in each checkout:

```sh
python3 tools/check/install_hooks.py
```

Requires Python 3.11+ and `rustfmt`. Installation preserves existing hooks and
chains an existing pre-commit hook after the formatting check. Reinstallation is
safe. `core.hooksPath` points to local wrappers in the Git directory.

The hook checks only added/changed staged Rust blobs, including the prospective
index used by `git commit --only`. It never formats files or changes the index.
Unstaged edits and child modules are not checked. Cargo editions come from staged
manifests; standalone harnesses default to their established edition 2021.

Format only files you own, with their actual crate edition, then stage them:

```sh
rustfmt --edition 2021 --config skip_children=true path/to/owned.rs
```

Do not run workspace-wide `cargo fmt` or `cargo fmt --all` in the shared checkout.
Formatting-only cleanup commits are allowed; coordinate shared files and keep
semantic work separate. Hooks check commits and cannot prevent editor or formatter
commands from rewriting unstaged files.

For an authorized formatting-only Rust cleanup, enable the commit-time guard:

```sh
C6_FORMAT_ONLY=1 git commit --only -m "Normalize owned Rust formatting" -- path/to/owned.rs
```

This mode compares each prospective staged Rust blob with HEAD after rustfmt
normalization, rejecting changes beyond formatter output. It rejects Rust file
additions, deletions and renames; excluded Rust paths must remain byte-identical.
The comparison uses Git's actual commit index, including `--only`, so a semantic
edit arriving after an earlier inspection cannot silently join that cleanup.
Ordinary feature commits omit the flag. The guard covers Rust content only;
review other owned paths separately and retain file ownership coordination.
