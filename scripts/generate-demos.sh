#!/usr/bin/env bash
# Generate the README showcase SVGs in docs/assets/.
#
# Sessions are recorded with asciinema to temp files (deleted afterwards),
# then converted with the local build. Only the .svg outputs are kept.
# Sessions must stay deterministic (no dates, timings, or absolute paths
# in visible output) and short (a few seconds each).
set -euo pipefail

OUT="docs/assets"
export TERM=xterm-256color

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

convert() { # name, session, -- nemasvg-args...
    local name="$1" session="$2"; shift 2
    [ "${1:-}" = "--" ] && shift
    echo "demo: $name"
    asciinema rec --overwrite "$TMPDIR/$name.cast" -q -c "$session"
    cargo run -q -- "$TMPDIR/$name.cast" "$OUT/$name.svg" "$@"
}

mkdir -p "$OUT"

# Hero: a tiny conversion story with spinner and payoff.
convert hero 'printf "$ nemasvg demo.cast demo.svg --window\n"; sleep 0.4; for f in ⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏; do printf "\r$f rendering 24 frames"; sleep 0.08; done; printf "\r\033[1;32m✓\033[0m demo.svg — 11 KB, sharp at any size      \n"; sleep 0.6' -- --window

# Style and palette showcase.
convert colors 'printf "\033[1mBold\033[0m \033[3mItalic\033[0m \033[4mUnderline\033[0m \033[9mStrike\033[0m \033[7mInverse\033[0m \033[2mFaint\033[0m\n"; sleep 0.3; printf "\033[31mRed \033[32mGreen \033[33mYellow \033[34mBlue \033[35mMagenta \033[36mCyan \033[37mWhite\033[0m\n"; sleep 0.3; printf "\033[38;5;200m256-pink\033[0m \033[38;2;255;165;0mtruecolor\033[0m \033[48;5;21mblue background\033[0m\n"; sleep 0.8' -- --window

# Download bar with right-anchored percentage.
convert progress 'for i in $(seq 0 4 100); do n=$(printf "%3d" "$i"); bar=$(printf "%0.s#" $(seq 1 $((i / 5)))); printf "\rdownloading [%-20s] %s%%" "$bar" "$n"; sleep 0.06; done; printf "\n\033[1;32mcomplete\033[0m\n"; sleep 0.5'

# Unicode, emoji, and box drawing.
convert unicode 'printf "CJK: \xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e\nemoji: \xf0\x9f\x8e\x89 \xe2\x9a\xa1 \xf0\x9f\xa6\x80\n"; sleep 0.3; printf "\xe2\x94\x8c\xe2\x94\x80\xe2\x94\x80\xe2\x94\x90\n\xe2\x94\x82 \xe2\x96\x88\xe2\x96\x92\xe2\x96\x91\xe2\x96\x91 \xe2\x94\x82\n\xe2\x94\x94\xe2\x94\x80\xe2\x94\x80\xe2\x94\x98\n"; sleep 0.8'

ls -la "$OUT"
