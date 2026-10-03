#!/usr/bin/env bash
# Build Planets.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see that script.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/planets.c" -o "$out/planets.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Planets.asi" \
         "$out/planets.obj" "$out/kernel32.lib"

echo "built $out/Planets.asi"
objdump -f "$out/Planets.asi" | sed -n '2,4p'
