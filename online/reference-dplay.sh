#!/usr/bin/env bash
# Put Microsoft's DirectPlay 8 and Online.asi into a test-bench clone.
#
#   online/reference-dplay.sh <session state.json>      [--no-asi]
#
# For a session started with `./a2test session start --no-launch`.  Proton's own
# dpnet.dll cannot host (README.md), so the reference the trace and our own
# transport are measured against is Microsoft's, from the DirectX redistributable
# that `winetricks directplay` uses.  It is not redistributable, so it is
# downloaded once into $A2_DATA/reference/directx/ (checked against winetricks'
# sha256) and only ever copied into a bench CLONE's prefix -- never the real one.
#
# In the clone: the DirectPlay DLLs replace the builtin symlinks in syswow64, a
# DllOverrides entry makes each one native, and regsvr32 registers them (which
# is what writes the TCP/IP service provider keys native dpnet looks for).
# Then Online.asi and Online.ini are built and copied into the clone's game.
set -euo pipefail

. "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/a2env.sh" >/dev/null
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

state="${1:?usage: reference-dplay.sh <session state.json> [--no-asi]}"
asi=1
[ "${2:-}" = "--no-asi" ] && asi=0

work="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["work"])' "$state")"
display="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["env"]["DISPLAY"])' "$state")"
prefix="$work/prefix"
[ -d "$prefix/drive_c" ] || { echo "no prefix at $prefix" >&2; exit 1; }
case "$prefix" in "$A2_PREFIX"*) echo "refusing: that is the real prefix" >&2; exit 1 ;; esac

# --- the reference DLLs, once ------------------------------------------------
ref="$A2_DATA/reference/directx"
dlls=(dplayx.dll dpnet.dll dpnhpast.dll dpnhupnp.dll dpwsockx.dll dpmodemx.dll
      dplaysvr.exe dpnsvr.exe dpvoice.dll dpvvox.dll dpvacm.dll)
if [ ! -f "$ref/dplay/dpnet.dll" ]; then
    mkdir -p "$ref/dplay"
    exe="$ref/directx_feb2010_redist.exe"
    if [ ! -f "$exe" ]; then
        # winetricks' URL: Microsoft retired the original, archive.org keeps it.
        curl -fL --retry 2 -o "$exe.part" \
            'https://web.archive.org/web/20100205000000id_/https://download.microsoft.com/download/E/E/1/EE17FF74-6C45-4575-9CF4-7FC2597ACD18/directx_feb2010_redist.exe'
        mv "$exe.part" "$exe"
    fi
    echo "f6d191e89a963d7cca34f169d30f49eab99c1ed3bb92da73ec43617caaa1e93f  $exe" | sha256sum -c --quiet
    # A self-extracting cabinet: cut the outer cabinet out, then dxnt.cab out of it.
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    python3 - "$exe" "$tmp/outer.cab" <<'EOF'
import struct, sys
d = open(sys.argv[1], 'rb').read()
i = d.find(b'MSCF\0\0\0\0')
while i != -1:
    size = struct.unpack_from('<I', d, i + 8)[0]
    if 1000 < size <= len(d) - i:
        open(sys.argv[2], 'wb').write(d[i:i + size])
        break
    i = d.find(b'MSCF\0\0\0\0', i + 1)
else:
    sys.exit('no cabinet in ' + sys.argv[1])
EOF
    bsdtar -xf "$tmp/outer.cab" -C "$tmp" dxnt.cab
    bsdtar -xf "$tmp/dxnt.cab" -C "$ref/dplay" "${dlls[@]}"
fi

# --- into the clone ------------------------------------------------------------
sys="$prefix/drive_c/windows/syswow64"
for f in "${dlls[@]}"; do
    rm -f "$sys/$f"
    cp "$ref/dplay/$f" "$sys/$f"
done

python3 - "$prefix/user.reg" <<'EOF'
import sys
p = sys.argv[1]
t = open(p, encoding='utf-8', errors='surrogateescape').read()
if '"dpnet"="native"' not in t:
    hdr = '[Software\\\\Wine\\\\DllOverrides]'
    names = ['dplayx', 'dpnet', 'dpnhpast', 'dpnhupnp', 'dpwsockx', 'dpmodemx',
             'dpvoice', 'dpvvox', 'dpvacm', 'dplaysvr.exe', 'dpnsvr.exe']
    add = ''.join(f'"{n}"="native"\n' for n in names)
    i = t.find(hdr)
    if i < 0:
        t += f'\n{hdr} 0\n' + add
    else:
        j = t.index('\n', i) + 1
        if t[j:].startswith('#time'):
            j = t.index('\n', j) + 1
        t = t[:j] + add + t[j:]
    open(p, 'w', encoding='utf-8', errors='surrogateescape').write(t)
EOF

umu="${A2_UMU:-$HOME/.config/heroic/tools/runtimes/umu/umu_run.py}"
for d in dplayx.dll dpnet.dll dpnhpast.dll dpnhupnp.dll; do
    env DISPLAY="$display" GAMEID=umu-0 STORE=gog PROTONPATH="$A2_PROTON" \
        WINEPREFIX="$prefix" STEAM_COMPAT_DATA_PATH="$prefix" \
        STEAM_COMPAT_INSTALL_PATH="$work/game" \
        STEAM_COMPAT_CLIENT_INSTALL_PATH="$HOME/.local/share/.steam/steam" \
        STEAM_COMPAT_APP_ID=0 UMU_RUNTIME_UPDATE=0 WINEDEBUG=-all \
        WINEDLLOVERRIDES='dplayx,dpnet,dpnhpast,dpnhupnp,dpwsockx=n' \
        python3 "$umu" regsvr32 /s "C:\\windows\\syswow64\\$d" >/dev/null 2>&1
done
# Wine's builtin already has the 64-bit key; the 32-bit one is regsvr32's doing.
grep -q 'Wow6432Node\\\\Microsoft\\\\DirectPlay8\\\\Service Providers\\\\DPNSPWinsockTCP' \
        "$prefix/system.reg" \
    || { echo "regsvr32 did not register the TCP/IP service provider" >&2; exit 1; }
echo "Microsoft DirectPlay installed in $prefix"

if [ "$asi" = 1 ]; then
    bash "$here/build.sh" >/dev/null
    cp "$here/build/Online.asi" "$here/Online.ini" "$work/game/"
    echo "Online.asi installed in $work/game (log: Online.log)"
fi
