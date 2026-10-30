#!/usr/bin/env bash
# Build Online.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from csdk/def/kernel32.def.  --kill-at is required; see that script.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

for lib in kernel32 ws2_32 ole32 user32; do
    llvm-dlltool -m i386 --kill-at -d "$here/../csdk/def/$lib.def" -l "$out/$lib.lib"
done

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/online.c" -o "$out/online.obj"     # includes peer.c

# The file's version resource (platform/versioninfo/README.md).
. "$here/../platform/versioninfo/version.sh"
"$here/../platform/versioninfo/build-res.sh" "$out/Online.res" Online.asi \
    "Armada II Refit: online multiplayer plugin" \
    "$(version "$here/CHANGELOG.md")" "$(version "$here/../CHANGELOG.md")"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Online.asi" \
         "$out/online.obj" "$out/Online.res" "$out/kernel32.lib" "$out/ws2_32.lib" "$out/ole32.lib" "$out/user32.lib"

echo "built $out/Online.asi"
objdump -f "$out/Online.asi" | sed -n '2,4p'
