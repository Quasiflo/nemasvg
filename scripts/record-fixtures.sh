#!/usr/bin/env bash
# Re-record the e2e fixture casts in tests/fixtures/.
#
# Requires asciinema (mise installs it from .config/mise.toml).
# Fixtures are recorded once and committed; the test suite only reads them,
# so recording nondeterminism (timestamps, scheduling) never affects tests.
# SVG goldens are re-blessed separately (see tests/e2e.rs).
#
# Rules for fixture sessions (keeps goldens stable across machines):
# - deterministic output only: no dates, no absolute paths, no network
# - small and fast: sub-second sleeps, a handful of output lines
set -euo pipefail

FIX="tests/fixtures"
export TERM=xterm-256color

rec() { # name, asciinema-args..., -- command...
	local name="$1"
	shift
	echo "recording $name"
	asciinema rec --overwrite "$@" "$FIX/$name.cast" -q
}

# SGR attributes and palettes from a real shell.
rec colors -c 'printf "\033[1;31mBold Red\033[0m normal \033[32mgreen\033[0m\n\033[3mitalic\033[0m \033[4munderline\033[0m \033[9mstrike\033[0m \033[7minverse\033[0m \033[2mfaint\033[0m\n\033[38;5;200m256\033[0m \033[38;2;255;165;0mtruecolor\033[0m \033[48;5;21mblue-bg\033[0m\n"; echo done'

# Alternate screen with dwell, via real shell escapes.
rec altscreen -c 'echo before; printf "\033[?1049h\033[HALT line1\nline2"; sleep 0.4; printf "\033[?1049l"; echo back'

# Full-screen app: real vim, relative filename for stable messages.
printf 'hello vim\nsecond line\n' >"$FIX/vim-work.txt"
rec vim -c "cd $FIX && vim -Nu NONE vim-work.txt -c 'normal! Goline3' -c 'sleep 300m' -c 'wq'"
rm -f "$FIX/vim-work.txt"

# Wide Unicode, emoji, box drawing (byte-literal, locale-independent).
rec unicode -c 'printf "CJK: \xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e end\nemoji: \xf0\x9f\x8e\x89 \xe2\x9a\xa1 ok\n\xe2\x94\x8c\xe2\x94\x80\xe2\x94\x90\n\xe2\x94\x94\xe2\x94\x80\xe2\x94\x98\n"'

# Cursor addressing, clear, erase; ends mid-line so the block cursor sits
# visibly after the partial command (real pty, ONLCR line endings).
rec cursor -c 'clear; tput cup 2 10; printf "HELLO"; tput cup 4 0; printf "line4"; tput el; printf "tail"; tput cup 0 0; printf "TOP"; printf "\n$ git sta"; sleep 0.3'

# Scrolling viewport with visible pacing (each line lands its own frames).
# shellcheck disable=SC2016 # -c string runs in the inner shell; $i/$(...) must stay literal here.
rec scroll -c 'for i in $(seq 1 30); do echo "scroll-line $i"; sleep 0.07; done'

# carriage-return progress bursts (FPS merging).
# shellcheck disable=SC2016 # -c string runs in the inner shell; $i/$(...) must stay literal here.
rec progress -c 'for i in $(seq 0 5 100); do printf "\rprogress %3d%%" "$i"; sleep 0.02; done; printf "\nfinished\n"'

# Idle gap with the limit embedded in the header.
rec idle -i 1 -c 'echo start; sleep 2; echo end'

echo "recorded: $(find "$FIX" -maxdepth 1 -name '*.cast' | wc -l) fixtures"
