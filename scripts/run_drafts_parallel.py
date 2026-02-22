#!/usr/bin/env python3
from __future__ import annotations

import argparse
import asyncio
import datetime as dt
import os
import shutil
from pathlib import Path


PROJECT_ROOT = Path(".").resolve()
PROJECT_META = PROJECT_ROOT / ".project" / "project.md"
DRAFTS_LIST = PROJECT_ROOT / ".project" / "drafts_list.yaml"
FEATURE_DIR = PROJECT_ROOT / ".project" / "feature"
CLEAR_DIR = PROJECT_ROOT / ".project" / "clear"
LOG_FILE = PROJECT_ROOT / ".project" / "log.md"


def now_iso() -> str:
    return dt.datetime.now().isoformat(timespec="seconds")


def append_failure_log(draft_name: str, reason: str) -> None:
    LOG_FILE.parent.mkdir(parents=True, exist_ok=True)
    with LOG_FILE.open("a", encoding="utf-8") as f:
        f.write(
            f"- task 이름: {draft_name} | 실패 시각: {now_iso()} | 실패 사유: {reason}\n"
        )


def discover_drafts() -> list[Path]:
    if not FEATURE_DIR.exists():
        return []
    return sorted(FEATURE_DIR.glob("*/draft.yaml"))


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8") if path.exists() else ""


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def parse_list_block(lines: list[str], key: str) -> tuple[list[str], tuple[int, int] | None]:
    start = None
    for i, line in enumerate(lines):
        if line.strip() == f"{key}:":
            start = i
            break
    if start is None:
        return [], None

    values: list[str] = []
    end = start
    for j in range(start + 1, len(lines)):
        line = lines[j]
        if not line.startswith("  - "):
            end = j
            break
        values.append(line.replace("  - ", "", 1).strip())
    else:
        end = len(lines)
    return values, (start, end)


def replace_list_block(lines: list[str], block: tuple[int, int] | None, key: str, values: list[str]) -> list[str]:
    new_block = [f"{key}:"] + [f"  - {v}" for v in values]
    if block is None:
        return lines + ([""] if lines and lines[-1] != "" else []) + new_block
    start, end = block
    return lines[:start] + new_block + lines[end:]


def mark_draft_completed(draft_folder: str) -> None:
    if not DRAFTS_LIST.exists():
        return

    lines = DRAFTS_LIST.read_text(encoding="utf-8").splitlines()
    feature_values, feature_block = parse_list_block(lines, "feature")
    planned_values, planned_block = parse_list_block(lines, "planned")

    if draft_folder in planned_values:
        planned_values = [v for v in planned_values if v != draft_folder]
    if draft_folder not in feature_values:
        feature_values.append(draft_folder)

    lines = replace_list_block(lines, feature_block, "feature", feature_values)
    lines = replace_list_block(lines, planned_block, "planned", planned_values)
    DRAFTS_LIST.write_text("\n".join(lines) + "\n", encoding="utf-8")


def move_to_clear(draft_path: Path) -> None:
    src_dir = draft_path.parent
    dst_dir = CLEAR_DIR / src_dir.name
    CLEAR_DIR.mkdir(parents=True, exist_ok=True)
    if dst_dir.exists():
        shutil.rmtree(dst_dir)
    shutil.move(str(src_dir), str(dst_dir))


def build_prompt(draft_path: Path) -> str:
    draft_text = read_text(draft_path)
    project_text = read_text(PROJECT_META)
    drafts_list_text = read_text(DRAFTS_LIST)
    return (
        "buil-code-parallel 실행.\n"
        "아래 3개 컨텍스트만 사용해 현재 draft의 task를 구현해.\n\n"
        "1) .project/project.md\n"
        f"{project_text}\n\n"
        "2) .project/drafts_list.yaml\n"
        f"{drafts_list_text}\n\n"
        f"3) task 단일 객체(대상 draft: {draft_path})\n"
        f"{draft_text}\n"
    )


async def run_one(
    sem: asyncio.Semaphore,
    draft_path: Path,
    command_template: str,
    timeout_sec: int,
    auto_yes_flag: str,
) -> tuple[str, bool]:
    draft_name = draft_path.parent.name
    prompt = build_prompt(draft_path)
    cmd = command_template.format(
        draft=str(draft_path),
        draft_name=draft_name,
        prompt=prompt.replace('"', '\\"'),
        auto_yes=auto_yes_flag,
    )

    async with sem:
        try:
            proc = await asyncio.create_subprocess_shell(cmd)
            await asyncio.wait_for(proc.wait(), timeout=timeout_sec)
            if proc.returncode != 0:
                append_failure_log(draft_name, f"process exit code {proc.returncode}")
                return draft_name, False
            mark_draft_completed(draft_name)
            move_to_clear(draft_path)
            return draft_name, True
        except asyncio.TimeoutError:
            append_failure_log(draft_name, f"timeout ({timeout_sec}s)")
            return draft_name, False
        except Exception as e:  # pragma: no cover
            append_failure_log(draft_name, str(e))
            return draft_name, False


async def main_async(args: argparse.Namespace) -> int:
    drafts = discover_drafts()
    if not drafts:
        print("no drafts found under .project/feature/*/draft.yaml")
        return 0

    sem = asyncio.Semaphore(args.max_parallel)
    auto_yes_flag = args.auto_yes_flag if args.auto_yes else ""
    jobs = [
        run_one(
            sem=sem,
            draft_path=draft_path,
            command_template=args.command_template,
            timeout_sec=args.timeout_sec,
            auto_yes_flag=auto_yes_flag,
        )
        for draft_path in drafts
    ]
    results = await asyncio.gather(*jobs)
    ok = sum(1 for _, success in results if success)
    fail = len(results) - ok
    print(f"completed={ok} failed={fail}")
    return 1 if fail else 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run one Codex process per draft in parallel."
    )
    parser.add_argument("--max-parallel", type=int, default=3)
    parser.add_argument("--timeout-sec", type=int, default=1800)
    parser.add_argument(
        "--auto-yes",
        action="store_true",
        help="Append auto-yes flag placeholder to command template.",
    )
    parser.add_argument(
        "--auto-yes-flag",
        default="-y",
        help="Flag string used when --auto-yes is enabled.",
    )
    parser.add_argument(
        "--command-template",
        default='codex exec {auto_yes} "{prompt}"',
        help=(
            "Shell command template. Available placeholders: "
            "{draft}, {draft_name}, {prompt}, {auto_yes}"
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    return asyncio.run(main_async(args))


if __name__ == "__main__":
    raise SystemExit(main())
