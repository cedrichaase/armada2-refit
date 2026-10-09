#!/usr/bin/env bash
# Build Sky.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see that script.
# -msse2 so float-to-int is cvttss2si, not a call to the CRT's _ftol2.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

# The sky shaders (sky.hlsl -> sky_shaders.h, committed).
"$here/../platform/d3d9/hlsl.sh" "$here/sky_shaders.h" "$here/sky.hlsl" \
    sky_vs:vs_3_0:k_sky_vs sky_ps:ps_3_0:k_sky_ps \
    bake_ps:ps_3_0:k_bake_ps draw_ps:ps_3_0:k_draw_ps

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra -msse2 \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/sky.c" -o "$out/sky.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Sky.asi" \
         "$out/sky.obj" "$out/kernel32.lib"

echo "built $out/Sky.asi"
