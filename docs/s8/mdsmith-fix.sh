#!/bin/bash
# hk fix command for docs/s8 notes: mdsmith fix, then the repo-mdsmith-fixer
# agent on what mdsmith cannot fix. Fails while findings remain.
# MDSMITH_AGENT=0 skips the agent. See "Docs and script style" in AGENTS.md.
set -euo pipefail

mdsmith fix "$@" && exit 0

agent="${MDSMITH_AGENT_FILE:-$HOME/.omp/agent/agents/repo-mdsmith-fixer.md}"
if [[ ${MDSMITH_AGENT:-1} == 0 || -n ${MDSMITH_AGENT_RUNNING:-} ]]; then
    exit 1
fi
if ! command -v omp > /dev/null || [[ ! -f $agent ]]; then
    echo "mdsmith-fix: omp or $agent missing; fix the findings above by hand" >&2
    exit 1
fi

model=$(awk 'NR > 1 && /^---$/ { exit } sub(/^model: */, "")' "$agent")
prompt=$(awk 'f; /^---$/ && ++n == 2 { f = 1 }' "$agent")
findings=$(mdsmith check --no-color -f json "$@" 2>&1 || true)
# The agent edits only the notes it is given, and "$@" can be the whole tree.
files=$(sed -n 's/^ *"file": "\(.*\.md\)",*$/\1/p' <<< "$findings" | sort -u | tr '\n' ' ')
if [[ -z $files ]]; then
    echo "mdsmith-fix: no note with findings to give the agent; fix them by hand" >&2
    exit 1
fi

echo "mdsmith-fix: asking repo-mdsmith-fixer ($model) to fix the findings above" >&2
MDSMITH_AGENT_RUNNING=1 omp -p --no-session --no-title --no-skills --no-rules \
    --model "$model" --append-system-prompt "$prompt" \
    --tools read,edit,grep,glob,bash --max-time "${MDSMITH_AGENT_MAX_TIME:-5m}" \
    "Repository root: $PWD
Files: $files
Findings from mdsmith check -f json, after mdsmith fix:
$findings" >&2 || echo "mdsmith-fix: the agent failed" >&2

mdsmith fix "$@"
