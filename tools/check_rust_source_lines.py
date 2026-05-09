#!/usr/bin/env python3
"""Fail when repository Rust source files exceed the line-count policy."""

from __future__ import annotations

import argparse
from pathlib import Path


DEFAULT_LIMIT = 1_000
EXCLUDED_DIRS = {".git", ".gnhf", ".venv", "target"}


def rust_sources(root: Path) -> list[Path]:
    """Return Rust source files that belong to the checked-in repository."""
    sources: list[Path] = []
    for path in root.rglob("*.rs"):
        if any(part in EXCLUDED_DIRS for part in path.relative_to(root).parts):
            continue
        sources.append(path)
    return sorted(sources)


def line_count(path: Path) -> int:
    """Count physical source lines without decoding file contents."""
    with path.open("rb") as source:
        return sum(1 for _ in source)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check that Rust source files stay at or below the line limit.",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=DEFAULT_LIMIT,
        help=f"maximum accepted line count per Rust source file (default: {DEFAULT_LIMIT})",
    )
    parser.add_argument(
        "root",
        nargs="?",
        default=".",
        help="repository root to scan (default: current directory)",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    oversized = [
        (lines, path.relative_to(root))
        for path in rust_sources(root)
        if (lines := line_count(path)) > args.limit
    ]

    if not oversized:
        print(f"checked {len(rust_sources(root))} Rust source files; limit {args.limit} lines")
        return 0

    print(f"Rust source files over {args.limit} lines:")
    for lines, path in sorted(oversized, reverse=True):
        print(f"{lines:5} {path}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
