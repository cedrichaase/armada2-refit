#!/usr/bin/env bash
# Build Nebulae.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
# Same toolchain and same reasons as menus/build.sh: no CRT
# (/nodefaultlib), Win32 prototypes declared in the source, import library
# generated from kernel32.def.  --kill-at is required; see that script.
# -msse2 so float-to-int is cvttss2si, not a call to the CRT's _ftol2.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

# The shaders (nebulae.hlsl -> nebulae_shaders.h, committed).
"$here/../platform/d3d9/hlsl.sh" "$here/nebulae_shaders.h" "$here/nebulae.hlsl" \
    neb_vs:vs_3_0:k_neb_vs neb_ps:ps_3_0:k_neb_ps \
    bake_vs:vs_3_0:k_bake_vs bake_ps:ps_3_0:k_bake_ps vol_ps:ps_3_0:k_vol_ps \
    vol_lr_ps:ps_3_0:k_vol_lr_ps quad_vs:vs_3_0:k_quad_vs zdown_ps:ps_3_0:k_zdown_ps comp_ps:ps_3_0:k_comp_ps zclear_ps:ps_3_0:k_zclear_ps

llvm-dlltool -m i386 --kill-at -d "$here/kernel32.def" -l "$out/kernel32.lib"

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra -msse2 \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/nebulae.c" -o "$out/nebulae.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Nebulae.asi" \
         "$out/nebulae.obj" "$out/kernel32.lib"

echo "built $out/Nebulae.asi"
