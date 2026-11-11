#!/usr/bin/env bash
# Lists upstream's Markdown, Swift and shell files, so the hooks can leave them
# alone: every other file of those types is the fork's own. Writes the list and the
# matching ignore block in .mdsmith.yml. With --check it writes nothing and fails
# when either is out of date.
#
# The root README.md is the fork's front page and is left off the list, so it is
# gated like the fork's own files.
#
# mdsmith matches an ignore pattern without a slash against the file name in every
# folder. Should a root file on the list share its name with a fork-owned file, the
# block re-includes that file.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

ref=upstream/main
list=docs/s8/upstream-files.txt
config=.mdsmith.yml
begin='  # BEGIN upstream files'
end='  # END upstream files'
adopted='README.md'

if ! git rev-parse --quiet --verify "$ref^{commit}" > /dev/null; then
    echo "upstream-files: no $ref here; add the upstream remote and fetch it" >&2
    [[ ${1:-} == --check ]] && exit 0
    exit 1
fi

new_list=$(mktemp)
block=$(mktemp)
new_config=$(mktemp)
trap 'rm -f "$new_list" "$block" "$new_config"' EXIT

git ls-tree -r --name-only "$ref" | grep -E '\.(md|swift|sh)$' | grep -vxF "$adopted" |
    LC_ALL=C sort > "$new_list"

git ls-files -- '*.md' | LC_ALL=C sort | awk -v list="$new_list" '
    BEGIN {
        while ((getline path < list) > 0) {
            if (path !~ /\.md$/) continue
            upstream[path] = 1
            if (path !~ /\//) root[path] = 1
            print "  - \"" path "\""
        }
    }
    !($0 in upstream) {
        name = $0
        sub(/.*\//, "", name)
        if (name in root) print "  - \"!" $0 "\""
    }
' > "$block"

awk -v begin="$begin" -v end="$end" -v block="$block" '
    $0 == begin {
        print
        while ((getline line < block) > 0) print line
        skip = 1
        found = 1
    }
    $0 == end { skip = 0 }
    !skip { print }
    END { if (!found) exit 1 }
' "$config" > "$new_config" || {
    echo "upstream-files: $config has no '$begin' line" >&2
    exit 1
}

if [[ ${1:-} == --check ]]; then
    if ! cmp -s "$new_list" "$list" || ! cmp -s "$new_config" "$config"; then
        echo "upstream-files: $list or $config is out of date;" \
            "run 'mise run upstream:files'" >&2
        exit 1
    fi
    exit 0
fi

cmp -s "$new_list" "$list" || cp "$new_list" "$list"
cmp -s "$new_config" "$config" || cp "$new_config" "$config"
