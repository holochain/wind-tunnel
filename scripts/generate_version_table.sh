#!/usr/bin/env bash
# Prints a Markdown version compatibility table to stdout by reading all
# semver version tags from git history without touching the working copy.
# Covers both old-style tags (0.x.0-alpha.N) and new-style tags (vX.Y.Z).
set -euo pipefail

extract_holochain_version() {
    local tag="$1"
    git show "${tag}:flake.lock" 2>/dev/null | python3 -c "
import json, sys
try:
    d = json.load(sys.stdin)
    nodes = d['nodes']
    # New format: nodes.holochain.original.ref = 'holochain-X.Y.Z'
    ref = nodes.get('holochain', {}).get('original', {}).get('ref', '')
    if ref.startswith('holochain-'):
        print(ref[len('holochain-'):])
    else:
        # Old format: actual version is on a secondary node referenced via nodes.versions
        hc_key = nodes.get('versions', {}).get('inputs', {}).get('holochain', 'holochain_2')
        ref2 = nodes.get(hc_key, {}).get('original', {}).get('ref', 'N/A')
        print(ref2[len('holochain-'):] if ref2.startswith('holochain-') else ref2)
except Exception:
    print('N/A')
" || echo "N/A"
}

extract_kitsune2_version() {
    local tag="$1"
    local version
    version=$(git show "${tag}:Cargo.lock" 2>/dev/null | awk '
        /^\[\[package\]\]/ { found=0 }
        /^name = "kitsune2"$/ { found=1 }
        found && /^version = / { gsub(/"/, "", $3); print $3; exit }
    ' || true)
    echo "${version:-N/A}"
}

# Collect semver tags: new-style (v*) and old-style (digit-prefixed), sorted newest first.
tags=$(git tag --sort=-creatordate --list 'v*' '[0-9]*')

if [[ -z "$tags" ]]; then
    echo "No version tags found." >&2
    exit 1
fi

echo "| Wind Tunnel | Holochain | Kitsune2 |"
echo "|-------------|-----------|----------|"

while IFS= read -r tag; do
    holochain=$(extract_holochain_version "$tag")
    kitsune2=$(extract_kitsune2_version "$tag")
    echo "| ${tag} | ${holochain} | ${kitsune2} |"
done <<< "$tags"
