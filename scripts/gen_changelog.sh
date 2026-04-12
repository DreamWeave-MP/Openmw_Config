#!/usr/bin/env bash
set -euo pipefail

REPO_URL="https://github.com/DreamWeave-MP/Openmw_Config"
OUTPUT="${1:-CHANGELOG.md}"

# Read all tags sorted newest-first
mapfile -t tags < <(git tag --sort=-version:refname 2>/dev/null)

emit_commits() {
    local range="$1"
    # read splits on first space: hash gets the short SHA, msg gets the rest
    git log "$range" --format="format:%h %s" | while read -r hash msg; do
        echo "- [$hash]($REPO_URL/commit/$hash) - $msg"
    done
}

{
    echo "# Changelog"
    echo ""

    if [ "${#tags[@]}" -eq 0 ]; then
        # No tags yet — dump everything under Unreleased
        echo "## Unreleased"
        echo ""
        emit_commits "HEAD"
        echo ""
    else
        # Commits since the latest tag
        unreleased=$(git log "${tags[0]}..HEAD" --format="format:%h %s")
        if [ -n "$unreleased" ]; then
            echo "## Unreleased"
            echo ""
            echo "$unreleased" | while read -r hash msg; do
                echo "- [$hash]($REPO_URL/commit/$hash) - $msg"
            done
            echo ""
        fi

        # One section per tag, newest first
        for i in "${!tags[@]}"; do
            tag="${tags[$i]}"
            echo "## $tag"
            echo ""

            if [ $(( i + 1 )) -lt "${#tags[@]}" ]; then
                emit_commits "${tags[$((i + 1))]}..${tag}"
            else
                # Oldest tag: show everything up to it
                emit_commits "$tag"
            fi

            echo ""
        done
    fi
} > "$OUTPUT"
