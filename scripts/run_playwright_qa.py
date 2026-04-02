#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import pathlib
import subprocess
import sys


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run a Playwright CLI command or Node-based verification script directly from the "
            "installed mono_Manager web workspace."
        )
    )
    parser.add_argument(
        "--web-root",
        required=True,
        help="Path to the web workspace that already contains package.json and node_modules.",
    )
    parser.add_argument(
        "command",
        nargs=argparse.REMAINDER,
        help="Command to run after '--', for example: node qa-check.mjs or playwright test",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    command = list(args.command)
    if command and command[0] == "--":
        command = command[1:]
    if not command:
        print("run_playwright_qa.py requires a command after '--'", file=sys.stderr)
        return 2

    web_root = pathlib.Path(args.web_root).resolve()
    node_modules = web_root / "node_modules"
    package_json = web_root / "package.json"
    if not package_json.exists():
        print(f"web workspace package.json not found: {package_json}", file=sys.stderr)
        return 2
    if not node_modules.exists():
        print(f"web workspace node_modules not found: {node_modules}", file=sys.stderr)
        return 2

    env = os.environ.copy()
    bin_dir = node_modules / ".bin"
    env["NODE_PATH"] = str(node_modules)
    env["PATH"] = f"{bin_dir}:{env.get('PATH', '')}" if bin_dir.exists() else env.get("PATH", "")
    env["ORC_QA_WEB_ROOT"] = str(web_root)
    env["ORC_QA_INSTALLED_WORKSPACE"] = str(web_root)

    completed = subprocess.run(command, cwd=web_root, env=env)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
