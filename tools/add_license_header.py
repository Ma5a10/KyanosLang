#!/usr/bin/env python3
# Copyright (c) 2026 ma5a10
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

"""
Add copyright notice and Mozilla Public License 2.0 (MPL-2.0) headers
to Rust (.rs) files in 'src' and 'tests' directories.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys

COPYRIGHT_LINE = "// Copyright (c) 2026 ma5a10"

MPL_NOTICE = """\
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/."""

FULL_HEADER = f"""\
{COPYRIGHT_LINE}
//
{MPL_NOTICE}
"""

OLD_HEADER_WITHOUT_COPYRIGHT = f"{MPL_NOTICE}\n"

IGNORED_DIR_NAMES = {
    "target",
    ".git",
    "node_modules",
    ".vscode",
    ".agents",
}

TARGET_DIR_NAMES = {"src", "tests"}


def is_target_file(path: Path) -> bool:
    """
    Check if a path is a Rust source file within a 'src' or 'tests' directory
    and not located in any ignored directories.
    """
    if path.suffix != ".rs":
        return False

    parts = set(path.parts)
    if parts.intersection(IGNORED_DIR_NAMES):
        return False

    # Check if any parent folder is named 'src' or 'tests'
    return bool(parts.intersection(TARGET_DIR_NAMES))


def has_valid_license_header(content: str) -> bool:
    """
    Check if the file content already has both the copyright notice and MPL-2.0 notice.
    """
    head = "\n".join(content.splitlines()[:20])
    has_copyright = COPYRIGHT_LINE in head
    has_mpl = ("Mozilla Public" in head or "MPL" in head) and (
        "https://mozilla.org/MPL/2.0/" in head or "http://mozilla.org/MPL/2.0/" in head
    )
    return has_copyright and has_mpl


def apply_header(content: str) -> str:
    """
    Insert or update the header to include the copyright notice and MPL-2.0 license.
    Preserves shebang lines if present.
    """
    # 1. If an existing MPL header without copyright is present, upgrade it
    if OLD_HEADER_WITHOUT_COPYRIGHT in content:
        return content.replace(OLD_HEADER_WITHOUT_COPYRIGHT, FULL_HEADER, 1)

    # 2. Otherwise insert at the beginning (after optional shebang)
    lines = content.splitlines(keepends=True)

    if lines and lines[0].startswith("#!") and not lines[0].startswith("#!["):
        shebang = lines[0]
        rest = "".join(lines[1:])
        return f"{shebang}\n{FULL_HEADER}\n{rest.lstrip()}"

    if content.strip():
        return f"{FULL_HEADER}\n{content.lstrip()}"
    return FULL_HEADER


def find_files(root_dir: Path) -> list[Path]:
    """
    Recursively find all candidate Rust files under root_dir.
    """
    target_files: list[Path] = []
    for path in root_dir.rglob("*.rs"):
        if is_target_file(path):
            target_files.append(path)
    return sorted(target_files)


def main() -> int:
    default_root = Path(__file__).resolve().parent.parent

    parser = argparse.ArgumentParser(
        description="Insert or verify copyright and MPL 2.0 header on .rs files in 'src' and 'tests' dirs."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=default_root,
        help=f"Project root directory to scan (default: {default_root})",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check only; exit with non-zero status if any file lacks the license/copyright header without modifying files.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print which files would be modified without actually writing to disk.",
    )
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Show detailed information for every inspected file.",
    )

    args = parser.parse_args()
    root_dir = args.root.resolve()

    if not root_dir.is_dir():
        print(f"Error: Directory '{root_dir}' does not exist.", file=sys.stderr)
        return 1

    files = find_files(root_dir)

    missing_header: list[Path] = []
    already_has_header: list[Path] = []

    for file_path in files:
        try:
            content = file_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            print(f"Warning: Could not decode UTF-8 for {file_path}, skipping.", file=sys.stderr)
            continue

        if has_valid_license_header(content):
            already_has_header.append(file_path)
            if args.verbose:
                print(f"[OK] {file_path.relative_to(root_dir)}")
        else:
            missing_header.append(file_path)

    print(f"Scanned {len(files)} target .rs file(s).")
    print(f"  - Valid copyright & license: {len(already_has_header)}")
    print(f"  - Missing or outdated:       {len(missing_header)}")

    if args.check:
        if missing_header:
            print("\nThe following files are missing valid copyright & MPL-2.0 headers:")
            for p in missing_header:
                print(f"  - {p.relative_to(root_dir)}")
            return 1
        print("All target files have valid headers.")
        return 0

    if not missing_header:
        print("No files need updating.")
        return 0

    if args.dry_run:
        print("\nDry-run mode: The following files would be updated:")
        for p in missing_header:
            print(f"  - {p.relative_to(root_dir)}")
        return 0

    # Apply headers
    updated_count = 0
    for file_path in missing_header:
        content = file_path.read_text(encoding="utf-8")
        new_content = apply_header(content)
        file_path.write_text(new_content, encoding="utf-8")
        updated_count += 1
        print(f"[UPDATED] {file_path.relative_to(root_dir)}")

    print(f"\nSuccessfully updated header in {updated_count} file(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
