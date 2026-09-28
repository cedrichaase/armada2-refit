#!/usr/bin/env bash
# Where the game is, and where this project's assets are. Every script in this
# repository resolves its paths through this file -- source it, don't copy the defaults:
#
#   A2_GAME     the game directory                 ~/Games/Heroic/Star Trek Armada II
#   A2_PREFIX   its Wine prefix                    ~/Games/Heroic/Prefixes/Star Trek Armada II
#   A2_PROTON   the Proton build that runs it      ~/.config/heroic/tools/proton/Proton-CachyOS-latest
#   A2_DATA     every asset: extracted stock art,  ${XDG_DATA_HOME:-~/.local/share}/armada2-refit
#               paid AI layers, builds -- never
#               in a working tree
#
# Each is taken from, in order: the environment; the config file
# ${XDG_CONFIG_HOME:-~/.config}/armada2-refit.conf (A2_CONF overrides), which holds
# KEY=value lines, `~` and `$HOME` expanded, `#` comments; the default above, which is
# where Heroic puts a GOG install. A2_GAME_DIR and A2_DIR, the names some scripts used
# before, are still read for A2_GAME.
#
# a2env.py is its Python half; keep the two in step.
#
#   ./a2env.sh      print what this machine resolves to

if [ -z "${A2_GAME:-}" ] && [ -n "${A2_GAME_DIR:-${A2_DIR:-}}" ]; then
  A2_GAME=${A2_GAME_DIR:-$A2_DIR}
fi
_a2_conf=${A2_CONF:-${XDG_CONFIG_HOME:-$HOME/.config}/armada2-refit.conf}
if [ -f "$_a2_conf" ]; then
  while IFS='=' read -r _a2_k _a2_v; do
    case "$_a2_k" in A2_GAME|A2_PREFIX|A2_PROTON|A2_DATA) ;; *) continue ;; esac
    [ -n "${!_a2_k:-}" ] && continue
    _a2_v=${_a2_v/#\~/$HOME}; _a2_v=${_a2_v//\$HOME/$HOME}
    printf -v "$_a2_k" '%s' "$_a2_v"
  done < "$_a2_conf"
fi
: "${A2_GAME:=$HOME/Games/Heroic/Star Trek Armada II}"
: "${A2_PREFIX:=$HOME/Games/Heroic/Prefixes/Star Trek Armada II}"
: "${A2_PROTON:=$HOME/.config/heroic/tools/proton/Proton-CachyOS-latest}"
: "${A2_DATA:=${XDG_DATA_HOME:-$HOME/.local/share}/armada2-refit}"
export A2_GAME A2_PREFIX A2_PROTON A2_DATA
unset _a2_conf _a2_k _a2_v

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  printf 'A2_GAME=%s\nA2_PREFIX=%s\nA2_PROTON=%s\nA2_DATA=%s\n' "$A2_GAME" "$A2_PREFIX" "$A2_PROTON" "$A2_DATA"
fi
