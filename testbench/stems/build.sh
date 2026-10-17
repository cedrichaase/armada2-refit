#!/usr/bin/env bash
# Build Stems.asi -- a 32-bit Windows DLL, cross-compiled on Linux as
# testbench/scene/build.sh builds Scene.asi: no CRT (/nodefaultlib), Win32 and
# DirectSound declared in the source, import library from kernel32.def.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/stems.c" -o "$out/stems.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Stems.asi" \
         "$out/stems.obj" "$out/kernel32.lib"

echo "built $out/Stems.asi"
