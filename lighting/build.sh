#!/usr/bin/env bash
# Build Lighting.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see that script.
# -msse2 so float-to-int is cvttss2si, not a call to the CRT's _ftol2.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

# The hull shaders (hull.hlsl -> hull_shaders.h, committed; Shaders=1).
"$here/../platform/d3d9/hlsl.sh" "$here/hull_shaders.h" "$here/hull.hlsl" \
    hull_vs:vs_3_0:k_hull_vs hull_ps:ps_3_0:k_hull_ps
# The planet shaders (planet.hlsl -> planet_shaders.h, committed; PlanetShaders=1).
"$here/../platform/d3d9/hlsl.sh" "$here/planet_shaders.h" "$here/planet.hlsl" \
    ground_ps:ps_3_0:k_ground_ps city_ps:ps_3_0:k_city_ps cloud_ps:ps_3_0:k_cloud_ps

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra -msse2 \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/lighting.c" -o "$out/lighting.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Lighting.asi" \
         "$out/lighting.obj" "$out/kernel32.lib"

echo "built $out/Lighting.asi"
