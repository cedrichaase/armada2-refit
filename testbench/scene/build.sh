#!/usr/bin/env bash
# Build Scene.asi -- a 32-bit Windows DLL, cross-compiled on Linux, the same way
# as qol/build.sh: no CRT (/nodefaultlib), Win32 prototypes declared in the
# source, import library from kernel32.def (--kill-at; see menus/build.sh).
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/scene.c" -o "$out/scene.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Scene.asi" \
         "$out/scene.obj" "$out/kernel32.lib"

echo "built $out/Scene.asi"
