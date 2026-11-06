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
