#!/usr/bin/env bash
# Build GridLayout.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as msaa/build.sh and menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def and user32.def.  --kill-at is required; see menus/build.sh.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"
llvm-dlltool -m i386 --kill-at -d "$here/user32.def"   -l "$out/user32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/grid.c" -o "$out/grid.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/GridLayout.asi" \
         "$out/grid.obj" "$out/kernel32.lib" "$out/user32.lib"

echo "built $out/GridLayout.asi"
objdump -f "$out/GridLayout.asi" | sed -n '2,4p'
