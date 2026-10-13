#!/usr/bin/env bash
# Build Specials.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see that script.
# -msse2 so float-to-int is cvttss2si, not a call to the CRT's _ftol2.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

# The shaders (specials.hlsl -> specials_shaders.h, committed).
"$here/../platform/d3d9/hlsl.sh" "$here/specials_shaders.h" "$here/specials.hlsl" \
    fx_vs:vs_3_0:k_fx_vs fx_ps:ps_3_0:k_fx_ps

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra -msse2 \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/specials.c" -o "$out/specials.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Specials.asi" \
         "$out/specials.obj" "$out/kernel32.lib"

echo "built $out/Specials.asi"
