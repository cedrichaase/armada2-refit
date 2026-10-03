#!/usr/bin/env bash
# Build D3DTrace.asi -- same toolchain as msaa/build.sh (no CRT, Win32
# prototypes in the source, import library from kernel32.def, --kill-at).
# -msse2 so float-to-int is cvttss2si, not a call to the CRT's _ftol2.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra -msse2 \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/d3dtrace.c" -o "$out/d3dtrace.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/D3DTrace.asi" \
         "$out/d3dtrace.obj" "$out/kernel32.lib"

echo "built $out/D3DTrace.asi"
