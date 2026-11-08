#!/usr/bin/env bash
# STA64 M1 presentation spike: builds M1Spike.asi (32-bit) and M1Host.exe (64-bit) with
# clang + lld-link, no CRT, import libraries from the .def files here. Throwaway.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"
for l in kernel32 user32; do
    llvm-dlltool -m i386 --kill-at -d "$here/def/$l-x86.def" -l "$out/$l-x86.lib"
    llvm-dlltool -m i386:x86-64 -d "$here/def/$l-x64.def" -l "$out/$l-x64.lib"
done
flags=(-O2 -Wall -Wextra -ffreestanding -fno-builtin -fno-stack-protector -mno-stack-arg-probe)
clang --target=i386-pc-windows-msvc "${flags[@]}" -c "$here/asi/m1spike.c" -o "$out/m1spike.obj"
lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 /out:"$out/M1Spike.asi" \
    "$out/m1spike.obj" "$out/kernel32-x86.lib" "$out/user32-x86.lib"
clang --target=x86_64-pc-windows-msvc "${flags[@]}" -c "$here/host/m1host.c" -o "$out/m1host.obj"
lld-link /machine:x64 /nodefaultlib /subsystem:windows /entry:start /out:"$out/M1Host.exe" \
    "$out/m1host.obj" "$out/kernel32-x64.lib" "$out/user32-x64.lib"
objdump -f "$out/M1Spike.asi" "$out/M1Host.exe" | grep -E 'file format|architecture'
