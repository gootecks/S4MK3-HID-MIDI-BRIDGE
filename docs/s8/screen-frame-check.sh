#!/bin/bash
# Attended screen phases 2 and 4 (#34, #35): send single S8 frames.
#   first          phase 2: black and F800 on the left panel, then the right
#   pattern ORDER  phase 4: full LITERAL test pattern, ORDER rgb or bgr
# Stops NIHardwareAgent with SIGKILL, sends each frame with
# examples/s8-screen-frame after an Enter, and restores the agent.
# Answering "stop" at any prompt skips to recovery. Raw logs go to
# docs/s8/captures/screen-PHASE-UTC/ so they survive `cargo clean`.
set -euo pipefail
cd "$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
# shellcheck source=docs/s8/screen-lib.sh
. docs/s8/screen-lib.sh

phase="${1:-}"
case "$phase" in
    first) steps=("black left" "f800 left" "black right" "f800 right"); order=rgb ;;
    pattern)
        order="${2:-}"
        [[ "$order" =~ ^(rgb|bgr)$ ]] || { echo "Usage: $0 pattern rgb|bgr"; exit 1; }
        steps=("pattern left" "pattern right") ;;
    *) echo "Usage: $0 first | pattern rgb|bgr"; exit 1 ;;
esac

screen_init "screen-$phase"
screen_preflight
screen_build debug
screen_start

note "Phase $phase, order $order. Djay open, no track playing."
screen_agent_stop

stopped=""
for step in "${steps[@]}"; do
    read -r frame side <<< "$step"
    name="$frame-$side"
    preview=()
    [ "$frame" != pattern ] || preview=(--preview "$run/$name.ppm")
    "$tool" "$frame" --screen "$side" --order "$order" --dump "$run/$name.bin" ${preview[@]+"${preview[@]}"} > /dev/null
    note "$name: $(wc -c < "$run/$name.bin" | tr -d ' ') bytes, sha256 $(shasum -a 256 "$run/$name.bin" | cut -d' ' -f1)"
    [ "$frame" = pattern ] || note "$name: $(xxd -p -c64 "$run/$name.bin")"
    ask "Next: send $name. Enter to send, or 'stop'."
    [ "$answer" != stop ] || { stopped="before $name"; break; }
    mark "$name"
    status=0
    "$tool" "$frame" --screen "$side" --order "$order" > "$run/$name.txt" 2>&1 || status=$?
    note "$name exit status $status:"
    tee -a "$run/notes.txt" < "$run/$name.txt"
    sleep 1
    snap "$name+1s"
    [ "$status" -eq 0 ] || { stopped="$name failed"; break; }
    ask "What do the panels show now? Take a photo. Describe, or 'stop'."
    [ "$answer" != stop ] || { stopped="after $name"; break; }
    ask "Press left PLAY once, then Enter. Did LEDs or audio change? Describe, or 'no'."
done
[ -z "$stopped" ] || note "Stopped: $stopped."

mark released
sleep 3
ask "Interface released, agent still stopped: what do the panels show? Photo."

screen_finish
