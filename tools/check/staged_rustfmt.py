#!/usr/bin/env python3
"""Check staged Rust blobs only. Never rewrite the index or working tree."""
import fnmatch
import pathlib
import subprocess
import sys
import tomllib


def git(*args):
    return subprocess.check_output(["git", *args])


def staged(path):
    result = subprocess.run(["git", "show", ":" + str(path)], capture_output=True)
    return result.stdout if result.returncode == 0 else None


def edition(path):
    # Standalone private harnesses use rustc --edition=2021.
    for directory in path.parents:
        content = staged(directory / "Cargo.toml")
        if content is None:
            continue
        manifest = tomllib.loads(content.decode())
        value = manifest.get("package", {}).get("edition")
        if isinstance(value, str):
            return value
        if isinstance(value, dict) and value.get("workspace"):
            for parent in directory.parents:
                workspace = staged(parent / "Cargo.toml")
                if workspace is not None:
                    value = tomllib.loads(workspace.decode()).get("workspace", {}).get("package", {}).get("edition")
                    if isinstance(value, str):
                        return value
            raise ValueError(f"cannot resolve workspace edition for {path}")
        if "package" in manifest:
            return "2015"  # Cargo's default for a package without an edition.
    return "2021"


def main():
    paths = git("diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z").split(b"\0")
    exclusions = (staged(pathlib.PurePosixPath(".rustfmt-exclude")) or b"").decode().splitlines()
    exclusions = [line.strip() for line in exclusions if line.strip() and not line.startswith("#")]
    failed = False
    for raw in paths:
        if not raw.endswith(b".rs"):
            continue
        path = pathlib.PurePosixPath(raw.decode())
        if any(fnmatch.fnmatchcase(str(path), pattern) for pattern in exclusions):
            continue
        content = staged(path)
        result = subprocess.run(
            ["rustfmt", "--edition", edition(path), "--config", "skip_children=true", "--emit", "stdout"],
            input=content, capture_output=True,
        )
        if result.returncode or result.stdout != content:
            failed = True
            print(f"Rust formatting check failed: {path}", file=sys.stderr)
            if result.stderr:
                sys.stderr.buffer.write(result.stderr)
            print(f"Format your owned file with: rustfmt --edition {edition(path)} --config skip_children=true '{path}'", file=sys.stderr)
    return int(failed)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        sys.exit(f"Rust formatting check could not run: {error}")
