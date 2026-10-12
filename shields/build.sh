#!/usr/bin/env bash
# Build Shields.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see that script.
# -msse2 so float-to-int is cvttss2si, not a call to the CRT's _ftol2.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

# The shaders (shields.hlsl -> shields_shaders.h, committed).
"$here/../platform/d3d9/hlsl.sh" "$here/shields_shaders.h" "$here/shields.hlsl" \
    shield_vs:vs_3_0:k_shield_vs shield_ps:ps_3_0:k_shield_ps \
    down_ps:ps_3_0:k_down_ps

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra -msse2 \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/shields.c" -o "$out/shields.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Shields.asi" \
         "$out/shields.obj" "$out/kernel32.lib"

echo "built $out/Shields.asi"
