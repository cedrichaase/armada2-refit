#!/usr/bin/env bash
# Compile the VERSIONINFO resource of one plugin, for lld-link to take beside its objects.
#   build-res.sh <out.res> <OriginalFilename> <FileDescription> <layer version> <project version>
# Company, product, copyright (LICENSE's line) and the repository URL are the same for
# every file; versioninfo.rc.in has them. Windows shows them in the file's Properties,
# and an unsigned DLL with none of them looks more suspicious to antivirus heuristics.
# README.md beside this script says who calls it.
set -euo pipefail

[ $# -eq 5 ] || { echo "usage: $0 <out.res> <OriginalFilename> <FileDescription> <layer version> <project version>" >&2; exit 2; }
out=$1 file=$2 desc=$3 fver=$4 pver=$5
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
semver='^[0-9]+\.[0-9]+\.[0-9]+$'
[[ $fver =~ $semver ]] || { echo "build-res.sh: layer version '$fver' is not X.Y.Z" >&2; exit 1; }
[[ $pver =~ $semver ]] || { echo "build-res.sh: project version '$pver' is not X.Y.Z" >&2; exit 1; }
copyright="$(grep -m1 '^Copyright' "$here/../../LICENSE"), MIT License"

# An rc string doubles its quotes; sed's replacement then needs \, & and | escaped.
rcstr () {
    local s=${1//\"/\"\"}
    s=${s//\\/\\\\}
    s=${s//&/\\&}
    printf '%s' "${s//|/\\|}"
}

rc=$(mktemp "$out.XXXXXX")   # beside the output: several builds may run at once
trap 'rm -f "$rc"' EXIT
sed -e "s|@FILEVERSION_NUM@|${fver//./,},0|" \
    -e "s|@PRODUCTVERSION_NUM@|${pver//./,},0|" \
    -e "s|@FILEVERSION@|$fver|" \
    -e "s|@PRODUCTVERSION@|$pver|" \
    -e "s|@FILEDESCRIPTION@|$(rcstr "$desc")|" \
    -e "s|@FILENAME@|$(rcstr "$file")|" \
    -e "s|@INTERNALNAME@|$(rcstr "${file%.*}")|" \
    -e "s|@COPYRIGHT@|$(rcstr "$copyright")|" \
    "$here/versioninfo.rc.in" > "$rc"
"${LLVM_RC:-llvm-rc}" -no-preprocess -fo "$out" -- "$rc"
