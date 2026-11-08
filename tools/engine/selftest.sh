#!/usr/bin/env bash
# The symbol-profile tools against their fixtures (tools/engine/README.md): the three
# resolvers (Python resolve.py, C csdk/a2engine.h through ctest.c, Rust crates/a2engine)
# on every fixtures/NN-*.toml, gen.py's output for the fixture set against the committed
# golden files, lint.py passing the good profiles and failing each bad one for its
# reason, the repository's profiles: generated files up to date, lint clean, and the
# offsets moved out of the sources (moved.py).
# Writes only to temp dirs. No game file is read.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fx="$here/fixtures"
fail=0
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; fail=1; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# 1. the resolvers
if python3 "$here/fixtures.py" run py c rs; then ok "fixtures (py, c, rs)"; else bad "fixtures"; fi

# 2. the generator's golden output
for lang in rust c; do
    ext=rs; [ "$lang" = c ] && ext=h
    python3 "$here/gen.py" --profiles "$fx/sets/basic" --emit "$fx/sets/basic/b1/p.toml" --lang "$lang" > "$tmp/p.$ext"
    if cmp -s "$tmp/p.$ext" "$fx/golden/p.$ext"; then ok "gen.py $lang"; else bad "gen.py $lang"; diff "$fx/golden/p.$ext" "$tmp/p.$ext" | head -20; fi
done

# 3. the validator on its good and bad profiles
if python3 "$here/lint.py" --selftest; then ok "lint.py fixtures"; else bad "lint.py fixtures"; fi

# 4. the repository's profiles
if python3 "$here/gen.py" --check; then ok "gen.py --check"; else bad "gen.py --check"; fi
if python3 "$here/lint.py"; then ok "lint.py"; else bad "lint.py"; fi

# 5. AR-3's equivalence test: every offset moved into a profile resolves to what the
#    source had, on gog-1.1
if python3 "$here/moved.py"; then ok "moved.py"; else bad "moved.py"; fi

# 6. the negative tests (the game-exe ones are skipped without a game)
if bash "$here/negative.sh" >"$tmp/neg.log" 2>&1; then ok "negative.sh"; else bad "negative.sh"; cat "$tmp/neg.log"; fi

# 7. fo-scan.py on synthetic images (no game file is read)
if python3 "$here/fo_scan_test.py" >"$tmp/fo.log" 2>&1; then ok "fo_scan_test.py"; else bad "fo_scan_test.py"; cat "$tmp/fo.log"; fi

[ "$fail" = 0 ] && echo "engine selftest: ok" || { echo "engine selftest: FAILED" >&2; exit 1; }
