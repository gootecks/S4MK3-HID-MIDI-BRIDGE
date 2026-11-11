#!/bin/bash
# Attended S8 LED output check (#21, #24): send the plan's LED reports one step at a time.
#   led-check.sh [mixer] [IDS...]        run the deck plan, or with `mixer` the 0x82 plan
#   led-check.sh dump [mixer]            print every step and its wire bytes; sends nothing
# Without IDS the non-optional steps run in plan order; IDS may name optional ones.
# Stops NIHardwareAgent with SIGKILL, sends each step with examples/s8-led-probe,
# asks the step's question and records the answer. Answering "stop" at any prompt
# skips to recovery. On every exit the LEDs are cleared and the agent is restored.
# Raw logs go to docs/s8/captures/led-check-UTC/ so they survive `cargo clean`.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
. docs/s8/screen-lib.sh

mode=run
name=led-check
plan=docs/s8/led-check-plan.json
allow=()
while [ $# -gt 0 ]; do
    case "$1" in
        dump) mode=dump ;;
        mixer)
            name=led-check-mixer
            plan=docs/s8/led-check-mixer-plan.json
            allow=(--allow-mixer)
            ;;
        *) break ;;
    esac
    shift
done

if [ "$mode" = dump ]; then
    cargo run --locked --quiet --example s8-led-probe -- --dump --plan "$plan" ${allow[@]+"${allow[@]}"}
    exit 0
fi

# step_field ID expect|ask: the field's text from the plan dump, on one line.
step_field() {
    awk -v id="$1" -v key="$2" '
        $0 == id { inside = 1; next }
        !inside { next }
        $0 == "" { exit }
        /^  expect: / { mode = "expect"; sub(/^  expect: /, "") }
        /^  ask: / { mode = "ask"; sub(/^  ask: /, "") }
        mode == key { sub(/^ +/, ""); text = text (text == "" ? "" : " ") $0 }
        END { print text }
    ' "$run/plan-dump.txt"
}

led_clear() {
    [ -n "${tool:-}" ] || return 0
    note "Clearing LEDs."
    "$tool" --clear ${allow[@]+"${allow[@]}"} 2>&1 | tee -a "$run/notes.txt" ||
        note "Clearing LEDs failed."
}

led_exit() {
    [ -n "$(agent_pid)" ] || led_clear
    screen_restore
}

screen_init "$name"
screen_preflight
screen_build debug s8-led-probe
probe_args=(--plan "$plan" ${allow[@]+"${allow[@]}"})
"$tool" --dump "${probe_args[@]}" > "$run/plan-dump.txt"
all=$("$tool" --list --all "${probe_args[@]}")
if [ $# -gt 0 ]; then
    ids=("$@")
    for id in "${ids[@]}"; do
        printf '%s\n' "$all" | grep -qx -- "$id" || {
            echo "No step $id in $plan."
            exit 1
        }
    done
else
    ids=()
    while IFS= read -r id; do
        ids+=("$id")
    done < <("$tool" --list "${probe_args[@]}")
fi
screen_start
trap led_exit EXIT

note "Plan $plan, steps: ${ids[*]}. Djay open, no track playing, Controller Editor open with panel labels visible."
screen_agent_stop

stopped=""
count=0
for id in "${ids[@]}"; do
    count=$((count + 1))
    printf '\n%s%s%s\n' "$cyan" "------------------------------------------------------------" "$reset"
    note "== $id ($count of ${#ids[@]})" "$cyan"
    note "Expect: $(step_field "$id" expect)" "$yellow"
    mark "$id"
    status=0
    "$tool" --step "$id" "${probe_args[@]}" > "$run/$id.txt" 2>&1 || status=$?
    if [ "$status" -eq 0 ]; then
        note "$id exit status $status:" "$dim"
    else
        note "$id exit status $status:" "$red"
    fi
    cat "$run/$id.txt" >> "$run/notes.txt"
    printf '%s%s%s\n' "$dim" "$(cat "$run/$id.txt")" "$reset"
    [ "$status" -eq 0 ] || {
        stopped="$id failed"
        break
    }
    row "$id: $(step_field "$id" ask) (or 'stop')" || {
        stopped="after $id"
        break
    }
done
[ -z "$stopped" ] || note "Stopped: $stopped." "$red"

led_clear
mark recover
[ -n "$(agent_pid)" ] || open -g "$bundle"
sleep 5
snap "recovered+5s"
ask "Are the panel labels and LEDs back?"
mark end
kill -INT "$recorder" 2> /dev/null || true
wait "$recorder" || true
trap - EXIT
note "Recorded in $run" "$green"
