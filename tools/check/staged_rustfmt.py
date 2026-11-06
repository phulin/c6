#!/usr/bin/env python3
"""Check staged Rust blobs only. Never rewrite the index or working tree."""
import fnmatch
import os
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


def normalized(content, path):
    # rustfmt can need a second pass for some existing harness source.
    for _ in range(5):
        result = subprocess.run(
            ["rustfmt", "--edition", edition(path), "--config", "skip_children=true", "--emit", "stdout"],
            input=content, capture_output=True,
        )
        if result.returncode:
            raise ValueError(f"cannot normalize {path}: {result.stderr.decode()}")
        if result.stdout == content:
            return content
        content = result.stdout
    raise ValueError(f"rustfmt did not converge for {path}")


def main():
    formatting_only = os.environ.get("C6_FORMAT_ONLY") == "1"
    paths = git("diff", "--cached", "--no-renames", "--name-only", "--diff-filter=" + ("ACMD" if formatting_only else "ACM"), "-z").split(b"\0")
    exclusions = (staged(pathlib.PurePosixPath(".rustfmt-exclude")) or b"").decode().splitlines()
    exclusions = [line.strip() for line in exclusions if line.strip() and not line.startswith("#")]
    failed = False
    for raw in paths:
        if not raw.endswith(b".rs"):
            continue
        path = pathlib.PurePosixPath(raw.decode())
        excluded = any(fnmatch.fnmatchcase(str(path), pattern) for pattern in exclusions)
        content = staged(path)
        if formatting_only:
            previous = subprocess.run(["git", "show", "HEAD:" + str(path)], capture_output=True)
            if previous.returncode or content is None:
                print(f"Formatting-only commit cannot add, delete or rename Rust files: {path}", file=sys.stderr)
                failed = True
                continue
            before = previous.stdout if excluded else normalized(previous.stdout, path)
            after = content if excluded else normalized(content, path)
            if before != after:
                print(f"Formatting-only commit contains changes beyond rustfmt: {path}", file=sys.stderr)
                failed = True
        if excluded:
            continue
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
