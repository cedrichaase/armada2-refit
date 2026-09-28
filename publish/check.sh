#!/usr/bin/env bash
# Is this tree publishable?  publish/check.sh [-C <repo>] [<commit>]   (default: the index)
#
# Fails, listing each offender, if the tree holds anything this repository must not
# redistribute (publish/README.md):
#   - any binary file, except a third-party one vendored under platform/vendor/<dir>/
#     whose licence expressly allows redistribution: that folder must hold the licence
#     text and a SOURCE.txt listing the file's sha256, and the content must match it.
#     Our own code is C, Python and shell, the recipes are text, and every build output
#     is derived and lives in A2_DATA -- so the rule needs no list of extensions a
#     renamed file could slip past;
#   - game file formats by extension, and anything under a target's, movie's or
#     backdrop's data directory (where assets lived before A2_DATA).
# .gitignore keeps these out by default; this catches `git add -f`.
set -uo pipefail
if [ "${1:-}" = -C ]; then cd "$2" || exit 2; shift 2; else cd "$(dirname "$0")/.."; fi
EMPTY=$(git hash-object -t tree /dev/null)
if [ $# -gt 0 ]; then
  numstat=$(git diff --numstat "$EMPTY" "$1")
  paths=$(git ls-tree -r --name-only "$1")
  rev="$1"
else
  numstat=$(git diff --cached --numstat "$EMPTY")
  paths=$(git ls-files)
  rev=""                                   # git show ":path" reads the index
fi
# vendored PATH -- a licensed third-party binary, listed by hash in its SOURCE.txt.
vendored () {
  local p="$1" dir="${1%/*}" sum
  local q lic=0
  case "$p" in platform/vendor/*/*) ;; *) return 1 ;; esac
  while IFS= read -r q; do
    case "${q,,}" in "${dir,,}"/*licen[cs]e*) lic=1 ;; esac
  done <<< "$paths"
  [ "$lic" = 1 ] || return 1
  sum=$(git show "$rev:$p" | sha256sum | cut -d' ' -f1)
  git show "$rev:$dir/SOURCE.txt" 2>/dev/null | grep -q -x -F "$sum  ${p##*/}"
}
bad=0
while IFS=$'\t' read -r a _ p; do
  [ "$a" = - ] || continue
  if vendored "$p"; then echo "vendored    $p"; else echo "binary      $p"; bad=1; fi
done <<< "$numstat"
while IFS= read -r p; do
  [ -n "$p" ] || continue
  case "$p" in
    archive/*|promo/*|textures/.scratch/*|textures/targets/*/*/*|\
    cutscenes/movies/*/*/*|menus/backdrops/*/*)
                                               echo "asset path  $p"; bad=1 ;;
  esac
  case "${p,,}" in
    *.tga|*.png|*.jpg|*.jpeg|*.webp|*.bmp|*.mp4|*.mkv|*.bik|*.wav|*.sod|*.spr|*.bzn|*.map|*.a2neb-backup)
                                               echo "game format $p"; bad=1 ;;
  esac
done <<< "$paths"
[ "$bad" = 0 ] && echo "publishable: ${1:-index}" || { echo "NOT publishable: ${1:-index}" >&2; exit 1; }
