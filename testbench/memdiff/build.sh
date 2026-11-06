#!/usr/bin/env bash
# Build MemDiff.asi -- a 32-bit Windows DLL, cross-compiled on Linux as the other
# bench-only plugins are (testbench/stems/build.sh): no CRT, the kernel32 calls of
# csdk/a2asi.h, the import library from csdk/def/kernel32.def.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"
exec 9>"$out/.build.lock"
flock 9

llvm-dlltool -m i386 --kill-at -d "$here/../../csdk/def/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/memdiff.c" -o "$out/memdiff.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/MemDiff.asi" \
         "$out/memdiff.obj" "$out/kernel32.lib"

echo "built $out/MemDiff.asi"
