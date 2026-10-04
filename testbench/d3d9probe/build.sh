#!/usr/bin/env bash
# Build D3D9Probe.asi -- the toolchain of testbench/d3dtrace/build.sh.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"; mkdir -p "$out"
llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"
clang --target=i386-pc-windows-msvc -O2 -Wall -Wextra -msse2 -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe -c "$here/d3d9probe.c" -o "$out/d3d9probe.obj"
lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 /out:"$out/D3D9Probe.asi" \
         "$out/d3d9probe.obj" "$out/kernel32.lib"
echo "built $out/D3D9Probe.asi"
