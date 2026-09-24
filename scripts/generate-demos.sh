#!/usr/bin/env bash
# Generate the README showcase SVGs in docs/assets/.
#
# Sessions are recorded with asciinema to temp files (deleted afterwards),
# then converted with the local build. Only the .svg outputs are kept.
# Sessions must stay deterministic (no dates, timings, or absolute paths
# in visible output) and short (a few seconds each). The hero demo streams
# asciiquarium.live, so regenerating it needs network access.
#
# Portability rules for session strings (learned the hard way):
# - outer strings are single-quoted: the inner shell sees them literally.
# - escapes only via printf '\033[..m' (instant, atomic); never $''.
# - newlines only via printf '\n'; a literal \n stays two characters.
# - no python/perl/node: character typing uses sed + read below, which is
#   code-point safe under a UTF-8 locale (verified for CJK and emoji).
set -euo pipefail

OUT="docs/assets"
export TERM=xterm-256color
# Fixed recording geometry: output SVGs must be identical no matter the
# caller's terminal size. The pty is forced below; --cols/--rows pins the
# canvas at convert time as well.
# The character typer below needs a UTF-8 locale for sed to split
# multibyte characters (CJK, emoji) rather than bytes.
for loc in C.UTF-8 en_US.UTF-8; do
	if locale -a 2>/dev/null | grep -qx "$loc"; then
		export LC_ALL="$loc"
		break
	fi
done
# Fail fast if splitting is still byte-wise on this machine.
[ "$(printf '%s' '日本🎉' | sed 's/./&\n/g' | wc -l)" -eq 3 ] || {
	echo "error: sed splits multibyte chars here (need a UTF-8 locale)" >&2
	exit 1
}

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

# typeout DELAY TEXT... — print text args character by character; there are
# no escape args by convention, escapes go through printf instead.
# NOTE: bare dollars below are literal (single-quoted). They survive the
# "$TYPEOUT" embedding below untouched, because expansion results are never
# rescanned — that is what the inner shell must receive. The sed program is
# single-quoted with a \n escape (a literal newline breaks BSD sed).
# shellcheck disable=SC2016 # single-quoted on purpose: inner shell expands $1/$2/$ch.
TYPEOUT='typeout() { printf "%s" "$2" | sed '"'"'s/./&\n/g'"'"' | while IFS= read -r ch; do printf "%s" "$ch"; sleep "$1"; done; }'

REC_COLS=80
REC_ROWS=24

convert() { # name, session, -- nemasvg-args...
	local name="$1" session="$2"
	shift 2
	[ "${1:-}" = "--" ] && shift
	echo "demo: $name"
	# Sessions run as executable files (not -c strings): dash does not apply
	# the ENOEXEC shell fallback, so a bare script path fails there.
	printf '#!/bin/sh\n%s\n' "$session" >"$TMPDIR/sess.sh"
	chmod +x "$TMPDIR/sess.sh"
	# Record under a fixed-size pty: asciinema inherits its terminal size,
	# so without this the header (and wrapping, tput output, canvas) would
	# follow whatever machine runs the script. `script` flags differ by OS.
	local run="stty cols $REC_COLS rows $REC_ROWS; exec asciinema rec --overwrite \"$TMPDIR/$name.cast\" -q -c \"$TMPDIR/sess.sh\""
	case "$(uname)" in
	Darwin) script -q /dev/null sh -c "$run" ;;
	*) script -qec "$run" /dev/null ;;
	esac
	cargo run -q -- "$TMPDIR/$name.cast" "$OUT/$name.svg" --cols "$REC_COLS" "$@"
}

mkdir -p "$OUT"

# Hero: convert command, then a live aquarium stream, taller window.
convert hero 'timeout 7 fireworks.sh' -- --window --rows 24

# Style showcase, typed letter by letter.
convert colors "$TYPEOUT; printf '\033[1m'; typeout 0.04 Bold; printf '\033[0m '; printf '\033[3m'; typeout 0.04 Italic; printf '\033[0m '; printf '\033[4m'; typeout 0.04 Underline; printf '\033[0m '; printf '\033[9m'; typeout 0.04 Strike; printf '\033[0m '; printf '\033[7m'; typeout 0.04 Inverse; printf '\033[0m '; printf '\033[2m'; typeout 0.04 Faint; printf '\033[0m\n'; sleep 0.3; printf '\033[31m'; typeout 0.04 Red; printf '\033[0m '; printf '\033[32m'; typeout 0.04 Green; printf '\033[0m '; printf '\033[33m'; typeout 0.04 Yellow; printf '\033[0m '; printf '\033[34m'; typeout 0.04 Blue; printf '\033[0m '; printf '\033[35m'; typeout 0.04 Magenta; printf '\033[0m '; printf '\033[36m'; typeout 0.04 Cyan; printf '\033[0m '; printf '\033[37m'; typeout 0.04 White; printf '\033[0m\n'; sleep 0.3; printf '\033[38;5;200m'; typeout 0.04 256-pink; printf '\033[0m '; printf '\033[38;2;255;165;0m'; typeout 0.04 truecolor; printf '\033[0m '; printf '\033[48;5;21m'; typeout 0.04 'blue background'; printf '\033[0m\n'; sleep 0.8" -- --window --rows 8

# Download bar with right-anchored percentage.
# shellcheck disable=SC2016 # session runs in the inner shell; $i/$n/$bar/$(...) must stay literal here.
convert progress 'for i in $(seq 0 4 100); do n=$(printf "%3d" "$i"); bar=$(printf "%0.s#" $(seq 1 $((i / 5)))); printf "\rdownloading [%-20s] %s%%" "$bar" "$n"; sleep 0.06; done; printf "\n\033[1;32mcomplete\033[0m\n"; sleep 0.5' -- --window --rows 8

# Unicode, emoji, and box drawing, typed letter by letter.
convert unicode "$TYPEOUT; typeout 0.05 'CJK: 日本語'; printf '\n'; typeout 0.05 'emoji: 🎉 ⚡ 🦀'; printf '\n'; sleep 0.3; typeout 0.05 '┌──┐'; printf '\n'; typeout 0.05 '│ █▒░░ │'; printf '\n'; typeout 0.05 '└──┘'; printf '\n'; sleep 0.8" -- --window --rows 8

ls -la "$OUT"
