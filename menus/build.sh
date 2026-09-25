#!/usr/bin/env bash
# Build Menus.asi -- a 32-bit Windows DLL, cross-compiled on Linux.
#
# No MSVC and no mingw here, and none is needed: the plugin declares the few
# Win32 prototypes it uses itself and links against import libraries generated
# from the .def files beside this script, with no CRT at all (/nodefaultlib).
# That is also why the source uses 32-bit integer arithmetic throughout --
# there is no _alldiv or _ftol to call.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$here/build"
mkdir -p "$out"

# --kill-at is not optional.  The .def files carry stdcall-decorated names
# (GetDC@4) because that is what the i386 object file references, but the
# real kernel32/user32/gdi32 export those names UNDECORATED.  Without -k the
# import table asks for "GetModuleFileNameA@12" and Wine aborts with
#   "Call from ... to unimplemented function KERNEL32.dll.GetModuleFileNameA@12"
# which the ASI loader reports only as "Unable to load Menus.asi. Error: 317".
for d in kernel32 user32 gdi32; do
    llvm-dlltool -m i386 --kill-at -d "$here/$d.def" -l "$out/$d.lib"
done

clang --target=i386-pc-windows-msvc \
      -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin \
      -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/menus.c" -o "$out/menus.obj"

lld-link /dll /machine:x86 /nodefaultlib /entry:DllMain@12 \
         /out:"$out/Menus.asi" \
         "$out/menus.obj" "$out/kernel32.lib" "$out/user32.lib" "$out/gdi32.lib"

# probe.exe: a test tool that lists the game's windows and injects input from
# inside its Wine session -- see probe.c and run-probe.sh.  Not installed.
clang --target=i386-pc-windows-msvc -O2 -Wall -Wextra \
      -ffreestanding -fno-builtin -fno-stack-protector -mno-stack-arg-probe \
      -c "$here/probe.c" -o "$out/probe.obj"
lld-link /subsystem:console /machine:x86 /nodefaultlib /entry:start@0 \
         /out:"$out/probe.exe" "$out/probe.obj" "$out/kernel32.lib" "$out/user32.lib"

echo "built $out/Menus.asi"
objdump -f "$out/Menus.asi" | sed -n '2,4p'
