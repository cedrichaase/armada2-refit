# Sourced, not run: version FILE prints the X.Y.Z of a changelog's newest
# "## X.Y.Z — date" heading. publish/package.sh and the plugins' build scripts read the
# layer and project versions with it.
version () { sed -n 's/^## \([0-9][0-9.]*\) .*/\1/p' "$1" | head -1; }
