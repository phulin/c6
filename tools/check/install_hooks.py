#!/usr/bin/env python3
"""Install tracked hooks locally, retaining and chaining existing hooks."""
import os
import pathlib
import shlex
import subprocess


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


root = pathlib.Path(git("rev-parse", "--show-toplevel"))
os.chdir(root)
gitdir = pathlib.Path(git("rev-parse", "--absolute-git-dir"))
destination = gitdir / "c6-hooks"
configured = subprocess.run(["git", "config", "--get", "core.hooksPath"], capture_output=True, text=True)
previous = pathlib.Path(configured.stdout.strip()) if configured.returncode == 0 else gitdir / "hooks"
if not previous.is_absolute():
    previous = root / previous
if previous.resolve() != destination.resolve():
    if destination.exists():
        raise SystemExit(f"Refusing to replace existing hook directory: {destination}")
    destination.mkdir()
    if previous.is_dir():
        for hook in previous.iterdir():
            if hook.is_file() and not hook.name.endswith(".sample"):
                (destination / ("pre-commit.previous" if hook.name == "pre-commit" else hook.name)).symlink_to(hook.resolve())
    wrapper = destination / "pre-commit"
    wrapper.write_text(
        '#!/bin/sh\nset -e\n'
        + shlex.quote(str(root / ".githooks/pre-commit")) + '\n'
        + 'if [ -x ' + shlex.quote(str(destination / "pre-commit.previous")) + ' ]; then\n'
        + '  exec ' + shlex.quote(str(destination / "pre-commit.previous")) + '\nfi\n'
    )
    wrapper.chmod(0o755)
subprocess.run(["git", "config", "--local", "core.hooksPath", str(destination)], check=True)
print(f"Installed staged Rust formatting check in {destination}; existing hooks preserved.")
