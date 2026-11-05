#!/usr/bin/env bash
# Where the game is, and where this project's assets are. Every script in this
# repository resolves its paths through this file -- source it, don't copy the defaults:
#
#   A2_GAME     the game's install folder          ~/Games/Heroic/Star Trek Armada II
#   A2_PREFIX   its Wine prefix                    ~/Games/Heroic/Prefixes/Star Trek Armada II
#   A2_HEROIC   Heroic's config directory          ${XDG_CONFIG_HOME:-~/.config}/heroic, else
#               (GamesConfig/, tools/)             the Flatpak's, if that one exists:
#                                                  ~/.var/app/com.heroicgameslauncher.hgl/config/heroic
#   A2_PROTON   the Proton build that runs it      $A2_HEROIC/tools/proton/Proton-CachyOS-latest
#   A2_DATA     every asset: extracted stock art,  ${XDG_DATA_HOME:-~/.local/share}/armada2-refit
#               paid AI layers, builds -- never
#               in a working tree
#   A2_PROFILE  the game (games/<id>.toml)         armada2 (the descriptor marked default)
#   A2_EXE      its executable (derived)           $A2_GAME/Armada2.exe
#
# Each is taken from, in order: the environment; the config file
# ${XDG_CONFIG_HOME:-~/.config}/armada2-refit.conf (A2_CONF overrides), which holds
# KEY=value lines, `~` and `$HOME` expanded, `#` comments, and a `[<id>]` section per
# other game; the default above, which the game's descriptor (games/<id>.toml) gives.
# A2_GAME_DIR and A2_DIR, the names some scripts used before, are still read for A2_GAME.
# The defaults shown are armada2's; A2_PROFILE=fo-roots (or --game) chooses another game.
#
# The values come from a2env.py, the one place that works them out (its docstring has
# the rules), so the two cannot drift. Shell variables set before sourcing count, as
# the environment does.
#
#   ./a2env.sh [--game ID]     print what this machine resolves to

_a2_here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
_a2_args=()
if [ "${BASH_SOURCE[0]}" = "$0" ]; then _a2_args=("$@"); fi
if _a2_out=$(A2_GAME="${A2_GAME:-}" A2_GAME_DIR="${A2_GAME_DIR:-}" A2_DIR="${A2_DIR:-}" \
             A2_PREFIX="${A2_PREFIX:-}" A2_HEROIC="${A2_HEROIC:-}" A2_PROTON="${A2_PROTON:-}" \
             A2_DATA="${A2_DATA:-}" A2_PROFILE="${A2_PROFILE:-}" A2_CONF="${A2_CONF:-}" \
             python3 "$_a2_here/a2env.py" "${_a2_args[@]}" --sh); then
  eval "$_a2_out"
else
  echo "a2env.sh: a2env.py could not resolve the game (see above)" >&2
  unset _a2_here _a2_args _a2_out
  return 1 2>/dev/null || exit 1
fi
unset _a2_here _a2_args _a2_out

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
  printf 'A2_GAME=%s\nA2_PREFIX=%s\nA2_HEROIC=%s\nA2_PROTON=%s\nA2_DATA=%s\nA2_PROFILE=%s\nA2_EXE=%s\n' \
    "$A2_GAME" "$A2_PREFIX" "$A2_HEROIC" "$A2_PROTON" "$A2_DATA" "$A2_PROFILE" "$A2_EXE"
fi
