#!/usr/bin/env bash
# Build binkw32.dll (the Bink replacement) and binktest.exe (its harness) --
# 32-bit Windows binaries, cross-compiled on Linux.  Same toolchain and same
# reasons as msaa/build.sh: no CRT (/nodefaultlib), Win32 prototypes
# declared in the source, import libraries generated from the .def files with
# --kill-at (the .def carries decorated names, the real DLLs export
# undecorated ones).
#
# -fno-omit-frame-pointer is load-bearing: BinkCopyToBuffer reads its
# caller's frame through the saved EBP to find the back buffer's size.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

for lib in kernel32 user32 winmm; do
    llvm-dlltool -m i386 --kill-at -d "$here/$lib.def" -l "$out/$lib.lib"
done

cflags=(--target=i386-pc-windows-msvc -O2 -Wall -Wextra -msse2
        -ffreestanding -fno-builtin -fno-omit-frame-pointer
        -fno-stack-protector -mno-stack-arg-probe)

clang "${cflags[@]}" -c "$here/binkproxy.c" -o "$out/binkproxy.obj"
clang "${cflags[@]}" -c "$here/binktest.c"  -o "$out/binktest.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/binkw32.dll" \
         "$out/binkproxy.obj" "$out/kernel32.lib" "$out/user32.lib" "$out/winmm.lib"

lld-link /machine:x86 /nodefaultlib /entry:start@0 /subsystem:console \
         /out:"$out/binktest.exe" \
         "$out/binktest.obj" "$out/kernel32.lib"

echo "built $out/binkw32.dll and $out/binktest.exe"
objdump -p "$out/binkw32.dll" | sed -n '/\[Ordinal\/Name Pointer\] Table/,/^$/p' | grep -o '_Bink[A-Za-z]*@[0-9]*'
