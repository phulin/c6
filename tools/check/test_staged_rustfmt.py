#!/usr/bin/env python3
"""Exercise hooks in a disposable repository, without touching caller state."""
import os
import pathlib
import shutil
import subprocess
import tempfile

SOURCE = pathlib.Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory(prefix="c6-hook-test-") as temporary:
    root = pathlib.Path(temporary)

    def run(*args, ok=True, formatting_only=False):
        environment = dict(os.environ)
        environment.pop("C6_FORMAT_ONLY", None)
        if formatting_only:
            environment["C6_FORMAT_ONLY"] = "1"
        result = subprocess.run(args, cwd=root, capture_output=True, text=True, env=environment)
        assert (result.returncode == 0) == ok, (args, result.stdout, result.stderr)
        return result

    run("git", "init", "-q")
    run("git", "config", "user.email", "hook-test@example.invalid")
    run("git", "config", "user.name", "Hook Test")
    for name in (".githooks/pre-commit", "tools/check/staged_rustfmt.py", "tools/check/install_hooks.py"):
        destination = root / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(SOURCE / name, destination)
    legacy = root / ".git/hooks/pre-commit"
    legacy.write_text("#!/bin/sh\nprintf legacy >> legacy-ran\n")
    legacy.chmod(0o755)
    run("python3", "tools/check/install_hooks.py")
    run("python3", "tools/check/install_hooks.py")
    (root / "Cargo.toml").write_text('[package]\nname="hook-test"\nversion="0.1.0"\nedition="2021"\n')
    rust = root / "main.rs"
    rust.write_text("fn main() {}\n")
    run("git", "add", "Cargo.toml", "main.rs")
    run("git", "commit", "--only", "-qm", "baseline", "--", "Cargo.toml", "main.rs")
    rust.write_text("fn main(){let x=1;}\n")
    run("git", "add", "main.rs")
    index_before = (root / ".git/index").read_bytes()
    work_before = rust.read_bytes()
    run("python3", "tools/check/staged_rustfmt.py", ok=False)
    assert index_before == (root / ".git/index").read_bytes()
    assert work_before == rust.read_bytes()
    rust.write_text("fn main() {\n    let _x = 1;\n}\n")
    run("git", "add", "main.rs")
    rust.write_text("invalid unstaged Rust")
    unrelated = root / "unrelated.rs"
    unrelated.write_text("also invalid unstaged Rust")
    run("python3", "tools/check/staged_rustfmt.py")
    # --only replaces the normal index entry with the selected worktree blob.
    run("git", "commit", "--only", "-qm", "reject malformed", "--", "main.rs", ok=False)
    rust.write_text("fn main() {\n    let _x = 2;\n}\n")
    run("git", "commit", "--only", "-qm", "formatted", "--", "main.rs")
    assert (root / "legacy-ran").read_text() == "legacylegacy"
    assert unrelated.read_text() == "also invalid unstaged Rust"
    # A commit-time guard sees the actual --only index, not an earlier inspection.
    rust.write_text("fn main() {\n    let _x = 3;\n}\n")
    run("git", "commit", "--only", "-qm", "reject semantic change", "--", "main.rs", formatting_only=True, ok=False)
    run("git", "commit", "--only", "-qm", "ordinary feature", "--", "main.rs")
    # Seed an unformatted committed source to exercise a real cleanup.
    rust.write_text("fn main(){let _x=3;}\n")
    run("git", "-c", "core.hooksPath=/dev/null", "commit", "--only", "-qm", "legacy formatting", "--", "main.rs")
    rust.write_text("fn main() {\n    let _x = 3;\n}\n")
    run("git", "add", "main.rs")
    rust.write_text("unrelated unstaged change to the same file")
    run("python3", "tools/check/staged_rustfmt.py", formatting_only=True)
    # --only must reject worktree semantics even with a safe normal index.
    rust.write_text("fn main() {\n    let _x = 4;\n}\n")
    run("git", "commit", "--only", "-qm", "reject late semantic edit", "--", "main.rs", formatting_only=True, ok=False)
    rust.write_text("fn main() {\n    let _x = 3;\n}\n")
    run("git", "commit", "--only", "-qm", "formatting cleanup", "--", "main.rs", formatting_only=True)
print("staged formatting, malformed input, --only, unstaged isolation, nonmutation, hook chaining, formatting-only semantic guard: PASS")
