#!/usr/bin/env python3
from __future__ import annotations

import pathlib
import subprocess
import sys


def build_command(argv: list[str]) -> list[str]:
    return [
        "cargo",
        "run",
        "--quiet",
        "--bin",
        "rc",
        "--",
        "run-playwright-qa",
        *argv,
    ]


def main() -> int:
    repo_root = pathlib.Path(__file__).resolve().parent.parent
    command = build_command(sys.argv[1:])
    completed = subprocess.run(command, cwd=repo_root)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
