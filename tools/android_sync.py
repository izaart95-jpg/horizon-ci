#!/usr/bin/env python3
"""Review the commits on `origin/android` since the recorded 1.8.9 sync point.

The Android fork's `android` branch is the source of platform, renderer and
account work; this repository keeps Minecraft 1.8.9 semantics on top of it.
This tool lists what is new, groups commits by how likely they are to need a
1.8.9 adaptation, and prints a checklist. It never changes the tree.

    python3 tools/android_sync.py
    python3 tools/android_sync.py --fetch
    python3 tools/android_sync.py --since <commit>
    python3 tools/android_sync.py --json

Read docs/android-sync.md before replaying anything.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / "docs" / "android-upstream.json"

# Paths whose behaviour depends on the Minecraft version. A commit touching one
# of these can merge cleanly and still be wrong for 1.8.9.
HIGH_RISK = (
    "crates/horizon-protocol/",
    "crates/horizon-client/src/net.rs",
    "crates/horizon-client/src/lan/",
    "crates/horizon-client/src/game/remote",
    "crates/horizon-client/src/game/combat",
    "crates/horizon-client/src/game/sound",
    "crates/horizon-client/src/game/attack",
    "crates/horizon-client/src/game/shield",
    "crates/horizon-client/src/game/consume.rs",
    "crates/horizon-client/src/game/potions.rs",
    "crates/horizon-client/src/game/trident.rs",
    "crates/horizon-client/src/save.rs",
    "crates/horizon-client/src/inventory",
    "crates/horizon-client/src/entities.rs",
    "crates/horizon-core/src/version",
    "crates/horizon-core/src/target.rs",
    "crates/horizon-world/src/chunk",
    "crates/horizon-world/src/datafix/",
    "crates/horizon-world/src/legacy_1_8_9.rs",
    "crates/horizon-world/src/level/",
    "crates/horizon-items/src/legacy_1_8_9.rs",
    "crates/horizon-items/src/blocks.rs",
    "crates/horizon-items/src/items.rs",
)

MEDIUM_RISK = (
    "crates/horizon-client-render/src/gfx",
    "crates/horizon-client-render/src/mesher",
    "crates/horizon-client-render/src/terrain",
    "crates/horizon-client-render/src/sprites.rs",
    "crates/horizon-client-render/src/packs.rs",
    "crates/horizon-client/src/game/",
    "crates/horizon-client/src/mobs/",
    "crates/horizon-items/",
    "crates/horizon-world/",
    "crates/horizon-client/src/world.rs",
    "crates/horizon-client/src/motion",
)


def git(*args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", "--no-pager", *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
    )
    if check and result.returncode != 0:
        raise SystemExit(
            f"git {' '.join(args)} failed:\n{result.stderr.strip()}"
        )
    return result.stdout


def sync_point(explicit: str | None) -> tuple[str, str]:
    if explicit:
        return explicit, "--since"
    try:
        record = json.loads(RECORD.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"cannot read {RECORD.relative_to(ROOT)}: {error}")
    commit = record.get("synced_origin_commit")
    if commit:
        return commit, str(RECORD.relative_to(ROOT))
    base = git("merge-base", "HEAD", "origin/android").strip()
    print(
        "warning: no synced_origin_commit recorded; using the merge base "
        f"{base[:12]}",
        file=sys.stderr,
    )
    return base, "merge-base"


def classify(files: list[str]) -> str:
    if any(path.startswith(prefix) for path in files for prefix in HIGH_RISK):
        return "HIGH"
    if any(path.startswith(prefix) for path in files for prefix in MEDIUM_RISK):
        return "MEDIUM"
    return "LOW"


def commits_between(base: str) -> list[dict]:
    raw = git(
        "log",
        "--reverse",
        "--format=%H%x09%s",
        f"{base}..origin/android",
        check=False,
    )
    commits = []
    for line in raw.splitlines():
        if not line.strip():
            continue
        sha, _, subject = line.partition("\t")
        files = [
            path
            for path in git("show", "--name-only", "--format=", sha).splitlines()
            if path.strip()
        ]
        commits.append(
            {
                "commit": sha,
                "short": sha[:10],
                "subject": subject,
                "risk": classify(files),
                "files": files,
            }
        )
    return commits


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fetch", action="store_true", help="fetch origin android first")
    parser.add_argument("--since", help="base commit instead of the recorded sync point")
    parser.add_argument("--json", action="store_true", help="machine-readable output")
    args = parser.parse_args()

    if args.fetch:
        git("fetch", "--no-tags", "origin", "android")

    tip = git("rev-parse", "--short", "origin/android").strip()
    base, source = sync_point(args.since)
    commits = commits_between(base)

    if args.json:
        print(
            json.dumps(
                {
                    "base": base,
                    "base_source": source,
                    "tip": tip,
                    "commits": commits,
                },
                indent=2,
            )
        )
        return 0

    order = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
    commits.sort(key=lambda commit: order[commit["risk"]])
    counts = {
        level: sum(1 for commit in commits if commit["risk"] == level)
        for level in ("HIGH", "MEDIUM", "LOW")
    }
    print(f"sync point: {base[:12]} ({source})")
    print(f"origin/android: {tip}")
    print(
        f"{len(commits)} commits to review: "
        f"{counts['HIGH']} high, {counts['MEDIUM']} medium, {counts['LOW']} low\n"
    )
    for commit in commits:
        print(f"{commit['risk']:<6} {commit['short']}  {commit['subject']}")
        shown = commit["files"][:4]
        for path in shown:
            print(f"       {path}")
        if len(commit["files"]) > len(shown):
            print(f"       ... {len(commit['files']) - len(shown)} more files")
    if commits:
        print(
            "\nNext: read the HIGH commits, replay on origin/android with "
            "docs/android-sync.md, then run the verification list."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
