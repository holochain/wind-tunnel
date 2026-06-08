#!/usr/bin/env python3
"""Update the version compatibility table in README.md for a given Wind Tunnel tag."""

import argparse
import json
import subprocess
import sys
import tomllib

SECTION_HEADER = "### Version compatibility"
TABLE_HEADER = "| Wind Tunnel | Holochain | Kitsune2 |"
TABLE_SEPARATOR = "|-------------|-----------|----------|"


def git_show(tag, path):
    result = subprocess.run(
        ["git", "show", f"{tag}:{path}"],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def extract_holochain_version(tag):
    content = git_show(tag, "flake.lock")
    if content is None:
        return "N/A"
    try:
        ref = json.loads(content)["nodes"]["holochain"]["original"]["ref"]
        if ref.startswith("holochain-"):
            return ref[len("holochain-"):]
    except (json.JSONDecodeError, KeyError, TypeError):
        pass
    return "N/A"


def extract_kitsune2_version(tag):
    content = git_show(tag, "Cargo.lock")
    if content is None:
        return "N/A"
    try:
        for package in tomllib.loads(content).get("package", []):
            if package.get("name") == "kitsune2":
                return package.get("version", "N/A")
    except tomllib.TOMLDecodeError:
        return "N/A"
    return "N/A"


def find_section_bounds(lines):
    start = None
    for i, line in enumerate(lines):
        if line.strip() == SECTION_HEADER:
            start = i + 1
            break
    if start is None:
        return None, None
    end = len(lines)
    for i in range(start, len(lines)):
        if lines[i].startswith("## "):
            end = i
            break
    return start, end


def parse_existing_rows(lines, start, end):
    rows = []
    in_table = False
    for line in lines[start:end]:
        stripped = line.strip()
        if stripped == TABLE_HEADER:
            in_table = True
            continue
        if stripped == TABLE_SEPARATOR:
            continue
        if in_table and stripped.startswith("|"):
            parts = [p.strip() for p in stripped.split("|")[1:-1]]
            if len(parts) == 3:
                rows.append(tuple(parts))
    return rows


def format_table(rows):
    lines = [TABLE_HEADER, TABLE_SEPARATOR]
    for wt, hc, k2 in rows:
        lines.append(f"| {wt} | {hc} | {k2} |")
    return lines


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True, help="Wind Tunnel git tag to add")
    parser.add_argument("--readme", default="README.md")
    args = parser.parse_args()

    holochain = extract_holochain_version(args.tag)
    kitsune2 = extract_kitsune2_version(args.tag)

    with open(args.readme) as f:
        lines = f.read().splitlines()

    start, end = find_section_bounds(lines)
    if start is None:
        print(f"ERROR: '{SECTION_HEADER}' not found in {args.readme}", file=sys.stderr)
        sys.exit(1)

    existing_rows = parse_existing_rows(lines, start, end)

    if any(row[0] == args.tag for row in existing_rows):
        print(f"Row for {args.tag} already present — nothing to do.")
        sys.exit(0)

    all_rows = [(args.tag, holochain, kitsune2)] + existing_rows

    new_lines = lines[:start] + [""] + format_table(all_rows) + [""] + lines[end:]

    with open(args.readme, "w") as f:
        f.write("\n".join(new_lines) + "\n")

    print(f"Added {args.tag} (Holochain {holochain}, Kitsune2 {kitsune2}).")


if __name__ == "__main__":
    main()
