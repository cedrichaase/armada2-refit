#!/usr/bin/env bash
# The settings tools against their fixtures (tools/settings/README.md): overlay.py's
# edit on every fixtures/NN-*.in (the spec crates/a2ini passes too, under cargo test),
# gen.py's output for the fixture tables against the committed golden files, lint.py
# passing the good tables and failing each bad one for its reason, the generated files
# of the repository up to date, the fixture C source compiled against csdk/a2asi.h (when
# clang is there), and bench-overlay.sh on a mock clone. Writes only to a temp dir.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fx="$here/fixtures"
tables="$fx/tables"
fail=0
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; fail=1; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# 1. the writer
if python3 "$here/overlay.py" --fixtures "$fx"; then ok "overlay.py fixtures"; else bad "overlay.py fixtures"; fi

# 2. the generator's golden output
python3 "$here/gen.py" --emit "$tables/good/settings.toml" --lang rs > "$tmp/good.rs"
cmp -s "$tmp/good.rs" "$tables/good/expected.rs" && ok "gen.py Rust table" || { bad "gen.py Rust table"; diff "$tables/good/expected.rs" "$tmp/good.rs" | head -20; }
for t in settings rules.settings; do
    out="$tables/good/${t//./_}_table.h"
    python3 "$here/gen.py" --emit "$tables/good/$t.toml" > "$tmp/t.h"
    cmp -s "$tmp/t.h" "$out" && ok "gen.py C table ($t.toml)" || { bad "gen.py C table ($t.toml)"; diff "$out" "$tmp/t.h" | head -20; }
done
if python3 "$here/gen.py" --check; then ok "gen.py --check"; else bad "gen.py --check"; fi

# 3. the lint
if python3 "$here/lint.py" "$tables"/good/*.toml >/dev/null; then ok "lint.py good"; else bad "lint.py good"; python3 "$here/lint.py" "$tables"/good/*.toml; fi
for d in "$tables"/bad-*/; do
    name="$(basename "$d")"
    want="$(cat "$d/expect.txt")"
    if out="$(python3 "$here/lint.py" "$d"*.toml 2>&1)"; then
        bad "lint.py $name passed"
    elif grep -qF -- "$want" <<<"$out"; then
        ok "lint.py $name"
    else
        bad "lint.py $name: wanted \"$want\", got:"
        echo "$out"
    fi
done
if python3 "$here/lint.py" >/dev/null; then ok "lint.py on the repository"; else bad "lint.py on the repository"; python3 "$here/lint.py"; fi

# 4. the C helpers and a generated table compile, with the header's static asserts
if command -v clang >/dev/null; then
    if clang --target=i386-pc-windows-msvc -O2 -Wall -Wextra -Werror -ffreestanding -fno-builtin \
            -fno-stack-protector -mno-stack-arg-probe -c "$tables/good/good.c" -o "$tmp/good.obj"; then
        ok "a2asi.h and the fixture table compile (i386)"
    else
        bad "a2asi.h and the fixture table compile (i386)"
    fi
else
    echo "skip a2asi.h compile (no clang)"
fi

# 5. bench-overlay.sh on a mock clone
mkdir -p "$tmp/work/game"
printf '{"work": "%s"}\n' "$tmp/work" > "$tmp/state.json"
bash "$here/bench-overlay.sh" "$tmp/state.json" Render3D.Modules:sky=0 UI.HUD:Minimap=0 --unset UI.HUD:Minimap \
    --into UIPlayer.ini HUD:BarHidden=1 >/dev/null
o="$tmp/work/game/RefitPlayer.ini"
if [ "$(python3 "$here/overlay.py" --get "$o" Render3D.Modules:sky)" = 0 ] \
        && ! python3 "$here/overlay.py" --get "$o" UI.HUD:Minimap >/dev/null \
        && [ "$(python3 "$here/overlay.py" --get "$tmp/work/game/UIPlayer.ini" HUD:BarHidden)" = 1 ]; then
    ok "bench-overlay.sh"
else
    bad "bench-overlay.sh"
    cat -A "$o"
fi

[ "$fail" = 0 ] && echo "tools/settings/selftest.sh: all pass" || echo "tools/settings/selftest.sh: FAILED"
exit "$fail"
