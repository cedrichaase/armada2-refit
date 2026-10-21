#!/usr/bin/env bash
# Build QOL.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh and hud/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see menus/build.sh.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/qol.c" -o "$out/qol.obj"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/rules.c" -o "$out/rules.obj"

# Each file's version resource (platform/versioninfo/README.md).
. "$here/../platform/versioninfo/version.sh"
"$here/../platform/versioninfo/build-res.sh" "$out/QOLRules.res" QOLRules.asi \
    "Armada II Refit: production queue and payment plugin" \
    "$(version "$here/CHANGELOG.md")" "$(version "$here/../CHANGELOG.md")"
"$here/../platform/versioninfo/build-res.sh" "$out/QOL.res" QOL.asi \
    "Armada II Refit: camera, selection and control-group plugin" \
    "$(version "$here/CHANGELOG.md")" "$(version "$here/../CHANGELOG.md")"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/QOLRules.asi" \
         "$out/rules.obj" "$out/QOLRules.res" "$out/kernel32.lib"
echo "built $out/QOLRules.asi"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/QOL.asi" \
         "$out/qol.obj" "$out/QOL.res" "$out/kernel32.lib"

echo "built $out/QOL.asi"
objdump -f "$out/QOL.asi" | sed -n '2,4p'
