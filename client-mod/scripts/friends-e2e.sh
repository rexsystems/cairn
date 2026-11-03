#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
version=${1:-26.3}
loader=${2:-fabric}
: "${CAIRN_SMOKE_SERVER:?Set CAIRN_SMOKE_SERVER to a local offline Minecraft server matching the requested version}"
case "$CAIRN_SMOKE_SERVER" in
    127.0.0.1:*|localhost:*) ;;
    *) echo 'The smoke server must be on loopback.' >&2; exit 1 ;;
esac
work=${CAIRN_SMOKE_DIR:-$(mktemp -d /tmp/cairn-friends-e2e.XXXXXX)}
mkdir -p "$work/launcher"
if [[ -n ${CAIRN_SMOKE_CACHE:-} ]]; then
    for directory in meta java; do
        if [[ ! -d "$work/launcher/$directory" ]]; then
            cp -a --reflink=auto "$CAIRN_SMOKE_CACHE/$directory" "$work/launcher/"
        fi
    done
fi
for command in Xvfb xdotool import python3 setsid; do command -v "$command" >/dev/null; done
cargo build --locked --manifest-path cosmetics-server/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml --example e2e
pids=()
cleanup() {
    for pid in "${pids[@]}"; do kill -TERM -- "-$pid" 2>/dev/null || true; done
    echo "Test artifacts: $work"
}
trap cleanup EXIT
setsid Xvfb -displayfd 3 -screen 0 2560x1440x24 -nolisten tcp 3>"$work/display" >"$work/xvfb.log" 2>&1 &
pids+=("$!")
for attempt in {1..100}; do [[ -s "$work/display" ]] && break; sleep .1; done
export DISPLAY=":$(cat "$work/display")" XDG_SESSION_TYPE=x11 SDL_VIDEODRIVER=x11
unset WAYLAND_DISPLAY
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
url="http://127.0.0.1:$port"
setsid env COSMETICS_BIND="127.0.0.1:$port" COSMETICS_DATA_DIR="$work/social" COSMETICS_DEV_INSECURE_AUTH=1 COSMETICS_ADMIN_TOKEN= COSMETICS_RATE_PER_MINUTE=10000 cosmetics-server/target/debug/cairn-cosmetics-server serve >"$work/social.log" 2>&1 &
pids+=("$!")
for attempt in {1..100}; do curl -fsS "$url/healthz" >/dev/null 2>&1 && break; sleep .1; done
export CAIRN_HOME="$work/launcher" E2E_CAIRN=1 E2E_FRIENDS=1 E2E_COSMETICS_URL="$url" E2E_HOLD=1800
for name in CairnAlice CairnBob; do
    server=""
    [[ "$name" == CairnBob ]] && server="$CAIRN_SMOKE_SERVER"
    setsid env E2E_PLAYER="$name" E2E_SERVER="$server" src-tauri/target/debug/examples/e2e "$version" "$loader" >"$work/$name.log" 2>&1 &
    game_pid=$!
    pids+=("$game_pid")
    for attempt in {1..1500}; do
        rg -q '^prepared ' "$work/$name.log" && break
        if ! kill -0 "$game_pid" 2>/dev/null; then cat "$work/$name.log" >&2; exit 1; fi
        sleep .2
    done
    rg -q '^prepared ' "$work/$name.log"
    if [[ "$loader" == forge && "$name" == CairnAlice ]]; then
        for attempt in {1..600}; do
            xdotool search --name Minecraft >/dev/null 2>&1 && break
            sleep .2
        done
    fi
done
python3 client-mod/scripts/friends-e2e.py "$url" "$version" "$work/screenshots" | tee "$work/result.json"
