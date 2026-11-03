#!/usr/bin/env bash
# The settings tools against their fixtures (tools/settings/README.md): the overlay's
# edit and rename on fixtures/ (cargo test of crates/a2ini, the one implementation),
# gen.py's output for the fixture tables against the committed golden files, lint.py
# passing the good tables and failing each case of fixtures/bad-cases.toml for its
# reason, the generated files of the repository up to date, the fixture C source
# compiled against csdk/a2asi.h (when clang is there), and bench-overlay.sh and
# move-keys.py on a mock game directory. Writes only to a temp dir.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fx="$here/fixtures"
tables="$fx/tables"
fail=0
ok() { echo "ok   $*"; }
bad() { echo "FAIL $*"; fail=1; }
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# 1. the writer: a2ini's own tests run the edit and rename fixtures
if CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" cargo test -q --manifest-path "$here/../../crates/a2ini/Cargo.toml" >"$tmp/cargo.log" 2>&1; then
    ok "a2ini cargo test (edit and rename fixtures)"
else
    bad "a2ini cargo test"; tail -20 "$tmp/cargo.log"
fi

# 2. the generator's golden output
python3 "$here/gen.py" --emit "$tables/good/settings.toml" --lang rs > "$tmp/good.rs"
cmp -s "$tmp/good.rs" "$tables/good/expected.rs" && ok "gen.py Rust table" || { bad "gen.py Rust table"; diff "$tables/good/expected.rs" "$tmp/good.rs" | head -20; }
for t in settings rules.settings; do
    out="$tables/good/${t//./_}_table.h"
    python3 "$here/gen.py" --emit "$tables/good/$t.toml" > "$tmp/t.h"
    cmp -s "$tmp/t.h" "$out" && ok "gen.py C table ($t.toml)" || { bad "gen.py C table ($t.toml)"; diff "$out" "$tmp/t.h" | head -20; }
done
if python3 "$here/gen.py" --check; then ok "gen.py --check"; else bad "gen.py --check"; fi

# 2b. fragments: a plugin's table and its modules' exported as one, in the order asked
frag="$tables/good-fragments"
if python3 "$here/gen.py" --rows "$frag/settings.toml" beta alpha | cmp -s - "$frag/expected-rows.txt"; then
    ok "gen.py --rows (fragments in module order)"
else
    bad "gen.py --rows (fragments in module order)"
fi
# Render3D's export keeps the order it had before its rows were split into fragments:
# the core's, then each module's in its priority (`fn priority` in the module's source).
r3d="$here/../../render3d"
order="$(python3 - "$r3d" <<'PY'
import os, re, sys
r3d = sys.argv[1]
mods = []
for m in sorted(os.listdir(os.path.join(r3d, 'src', 'mods'))):
    f = os.path.join(r3d, 'src', 'mods', m, 'mod.rs')
    if os.path.isfile(f):
        g = re.search(r'fn priority\(&self\) -> i32 \{\s*(-?\d+)\s*\}', open(f).read())
        if g:
            mods.append((int(g.group(1)), m))
print(' '.join(m for _, m in sorted(mods)))
PY
)"
# shellcheck disable=SC2086
if [ -n "$order" ] && python3 "$here/gen.py" --rows "$r3d/settings.toml" $order | cmp -s - "$fx/render3d-rows.txt"; then
    ok "Render3D_Settings row order unchanged ($order)"
else
    bad "Render3D_Settings row order (modules: $order)"
    # shellcheck disable=SC2086
    python3 "$here/gen.py" --rows "$r3d/settings.toml" $order | diff "$fx/render3d-rows.txt" - | head -20 || true
fi

# 3. the lint
if python3 "$here/lint.py" "$tables"/good/*.toml >/dev/null; then ok "lint.py good"; else bad "lint.py good"; python3 "$here/lint.py" "$tables"/good/*.toml; fi
mapfile -t fragtables < <(find "$frag" -name '*settings.toml' | sort)
if python3 "$here/lint.py" "${fragtables[@]}" >/dev/null; then ok "lint.py good-fragments"; else bad "lint.py good-fragments"; python3 "$here/lint.py" "${fragtables[@]}"; fi
python3 - "$here" "$tmp" <<'PY' || fail=1
import os, subprocess, sys, tomllib
here, tmp = sys.argv[1:3]
with open(os.path.join(here, 'fixtures', 'bad-cases.toml'), 'rb') as f:
    cases = tomllib.load(f)['case']
bad = 0
for c in cases:
    with open(os.path.join(here, 'fixtures', 'tables', 'good', 'settings.toml')) as f:
        text = f.read()
    if 'replace' in c:
        old, new = (x.replace('\\n', '\n') for x in c['replace'])
        assert old in text, c['name']
        if 'fill' in c:
            new = new.replace(c['fill'][0], 'x' * c['fill'][1])
        text = text.replace(old, new, 1)
    text += c.get('append', '')
    d = os.path.join(tmp, 'case-' + c['name'])
    os.makedirs(d)
    files = {'settings.toml': text, **c.get('files', {})}
    for n, t in files.items():
        with open(os.path.join(d, n), 'w') as f:
            f.write(t)
    r = subprocess.run([sys.executable, os.path.join(here, 'lint.py')] + [os.path.join(d, n) for n in sorted(files)],
                       capture_output=True, text=True)
    out = r.stdout + r.stderr
    if r.returncode == 0:
        print(f"FAIL lint.py case {c['name']} passed"); bad += 1
    elif c['expect'] not in out:
        print(f"FAIL lint.py case {c['name']}: wanted \"{c['expect']}\", got:\n{out}"); bad += 1
    else:
        print(f"ok   lint.py case {c['name']}")
sys.exit(1 if bad else 0)
PY
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

# 6. move-keys.py: a plugin-ini key moved by a row's moved_from, once
mv="$tmp/move"
mkdir -p "$mv/game" "$mv/repo/layer"
cat > "$mv/repo/layer/settings.toml" <<'EOT'
plugin = "New"
file = "New.ini"
shipped = "layer/New.ini"

[[row]]
name = "moved"
section = "Sec"
key = "Key"
kind = "bool"
default = "0"
apply = "live"
scope = "local"
tab = "graphics"
label = "Moved"
help = ""
moved_from = "Old.OSec.OKey"
EOT
printf '[Sec]\nKey=0\n' > "$mv/repo/layer/New.ini"
printf '; keep\r\n[OSec]\r\nOKey=1\r\nOther=2\r\n' > "$mv/game/Old.ini"
printf '[Sec]\nKey=0\n' > "$mv/game/New.ini"
python3 "$here/move-keys.py" --repo "$mv/repo" "$mv/game" >"$tmp/move.log"
if [ "$(python3 "$here/overlay.py" --get "$mv/game/New.ini" Sec:Key)" = 1 ] \
        && ! python3 "$here/overlay.py" --get "$mv/game/Old.ini" OSec:OKey >/dev/null \
        && [ "$(python3 "$here/overlay.py" --get "$mv/game/Old.ini" OSec:Other)" = 2 ] \
        && grep -q 'moved Old.ini \[OSec\] OKey' "$tmp/move.log"; then
    python3 "$here/move-keys.py" --repo "$mv/repo" "$mv/game" >"$tmp/move2.log"
    if [ "$(python3 "$here/overlay.py" --get "$mv/game/New.ini" Sec:Key)" = 1 ] && ! grep -q moved "$tmp/move2.log"; then
        ok "move-keys.py (moved once, then nothing)"
    else
        bad "move-keys.py second run"; cat "$tmp/move2.log"
    fi
else
    bad "move-keys.py"; cat "$tmp/move.log"
fi

[ "$fail" = 0 ] && echo "tools/settings/selftest.sh: all pass" || echo "tools/settings/selftest.sh: FAILED"
exit "$fail"
