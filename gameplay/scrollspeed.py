#!/usr/bin/env python3
"""Read and set Armada II's map-scroll knobs without the in-game options screen.

The options screen is the supported way to change the two per-device speeds, but it is
a GDI dialog that is not always usable (see menus/README.md).
This reaches the same two values directly, plus the RTS_CFG.h knobs the UI never
exposed at all.

    gameplay/scrollspeed.py                      # show current values
    gameplay/scrollspeed.py --mouse 5            # edge-scroll speed
    gameplay/scrollspeed.py --coefficient 400000 # every pan path at once
    gameplay/scrollspeed.py --revert             # back to the .a2neb-backup copies

Both files are read at LAUNCH, so nothing applies to a running game.
Full derivation, and what each knob actually multiplies, is in gameplay/README.md.
"""

import argparse
import os
import re
import shutil
import subprocess
import sys

GAME = os.environ.get("A2_DIR", "/home/cedric/Games/Heroic/Star Trek Armada II")
PRF = "ARMADA.PRF"
CFG = "RTS_CFG.h"
BAK = ".a2neb-backup"

# ARMADA.PRF line 2 is eleven whitespace-separated values; these two indices are the
# scroll speeds. Confirmed against both GameConfiguration::LoadProfile (0x53dc00) and
# its serializer (0x53c930) -- read order and write order agree.
PRF_MOUSE, PRF_KEYBOARD = 2, 3

# name -> (slider range, slider-from-value). The options screen stores mouse as
# slider/10 and keyboard as slider+1, so these invert back to a slider position.
SLIDER = {
    "mouse":    ((1, 50), lambda v: v * 10.0),
    "keyboard": ((1, 20), lambda v: v - 1.0),
}

CFG_KEYS = {
    "coefficient": ("SCROLL_COEFFICIENT", "float", "scales every pan path"),
    "max": ("MAX_SCROLL_SPEED", "float", "ceiling a held scroll ramps to"),
    "initial": ("INITIAL_SCROLL_SPEED", "float", "ramp floor; ramp gains +1.0/sec"),
    "accel": ("SCROLL_ACCELERATION", "int", "multiplier over the whole ramp"),
    "border": ("SCROLL_BORDER_WIDTH", "int", "edge band in pixels"),
    "fastscroll": ("FASTSCROLL_COEFFICIENT", "float", "right-drag only, gets no ramp"),
}


def path(name):
    p = os.path.join(GAME, name)
    if not os.path.exists(p):
        sys.exit("not found: %s\nSet A2_DIR if the game lives elsewhere." % p)
    return p


def game_running():
    """True if Armada2.exe is live.

    Matches on the matched process's own command line, skipping shells -- a plain
    `pgrep -f Armada2.exe` also matches the shell that is running the pgrep, which
    is a self-match that has already produced one wrong conclusion here.
    """
    try:
        out = subprocess.run(["pgrep", "-a", "-f", "Armada2"],
                             capture_output=True, text=True).stdout
    except (FileNotFoundError, OSError):
        return False
    for line in out.splitlines():
        body = line.split(None, 1)[1] if " " in line else ""
        if "pgrep" in body or "/bash" in body or "/sh" in body:
            continue
        return True
    return False


def backup(p):
    b = p + BAK
    if not os.path.exists(b):
        shutil.copy2(p, b)


def read_prf():
    lines = open(path(PRF), "rb").read().split(b"\r\n")
    return lines, lines[1].split(b" ")


def write_prf(lines, fields):
    p = path(PRF)
    backup(p)
    lines[1] = b" ".join(fields)
    open(p, "wb").write(b"\r\n".join(lines))


def cfg_pattern(key):
    return re.compile(
        r"^(\s*(?:float|int)\s+%s\s*=\s*)([0-9.]+)(\s*;)" % re.escape(key), re.M)


def cfg_read(p):
    """Read RTS_CFG.h keeping its CRLF endings verbatim.

    newline="" on BOTH the read and the write is what preserves them. A plain text-mode
    read applies universal newlines and silently turns \\r\\n into \\n, so writing it back
    -- even with newline="" -- rewrites the whole file as LF.
    """
    with open(p, encoding="latin-1", newline="") as fh:
        return fh.read()


def cfg_write(p, text):
    with open(p, "w", encoding="latin-1", newline="") as fh:
        fh.write(text)


def cfg_get(text, key):
    m = cfg_pattern(key).search(text)
    return m.group(2) if m else None


def cfg_set(text, key, kind, value):
    val = "%d" % int(float(value)) if kind == "int" else "%s" % float(value)
    pat = cfg_pattern(key)
    if not pat.search(text):
        sys.exit("%s not found in %s" % (key, CFG))
    return pat.sub(lambda m: m.group(1) + val + m.group(3), text, count=1)


def fmt(value):
    return "%d" % value if float(value).is_integer() else "%s" % value


def show():
    _, f = read_prf()
    print("%s  (per-user; what the options sliders write)" % PRF)
    for name, idx in (("mouse", PRF_MOUSE), ("keyboard", PRF_KEYBOARD)):
        (lo, hi), to_slider = SLIDER[name]
        s = to_slider(float(f[idx]))
        note = "" if lo <= s <= hi else "   [outside the slider's %d-%d range]" % (lo, hi)
        print("  %-9s %-8s  slider %g of %d%s" % (name, f[idx].decode(), s, hi, note))

    text = cfg_read(path(CFG))
    print("\n%s  (shared; hashed across clients in multiplayer)" % CFG)
    for flag, (key, _kind, why) in CFG_KEYS.items():
        print("  %-12s %-10s %s" % (flag, cfg_get(text, key), why))
    if game_running():
        print("\nNOTE: the game is running. Both files are read at launch.")


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--mouse", type=float,
                    help="edge-scroll speed (slider 1-50 => 0.1-5.0)")
    ap.add_argument("--keyboard", type=float,
                    help="arrow-key speed (slider 1-20 => 2.0-21.0)")
    for flag, (key, _kind, why) in CFG_KEYS.items():
        ap.add_argument("--" + flag, type=float, help="%s -- %s" % (key, why))
    ap.add_argument("--revert", action="store_true",
                    help="restore both .a2neb-backup files")
    a = ap.parse_args()

    if a.revert:
        for n in (PRF, CFG):
            p, b = path(n), path(n) + BAK
            if os.path.exists(b):
                shutil.copy2(b, p)
                print("reverted", n)
            else:
                print("no backup for", n)
        return

    touched = False

    prf_changes = [(PRF_MOUSE, a.mouse, "mouse"), (PRF_KEYBOARD, a.keyboard, "keyboard")]
    if any(v is not None for _, v, _ in prf_changes):
        lines, f = read_prf()
        for idx, val, name in prf_changes:
            if val is None:
                continue
            (lo, hi), to_slider = SLIDER[name]
            s = to_slider(val)
            if not lo <= s <= hi:
                print("warn: %s %g is outside the slider's range; the options screen "
                      "will clamp it if you ever open it" % (name, val))
            f[idx] = fmt(val).encode()
            print("%s -> %s" % (name, f[idx].decode()))
        write_prf(lines, f)
        touched = True

    cfg_changes = [(k, getattr(a, k)) for k in CFG_KEYS if getattr(a, k) is not None]
    if cfg_changes:
        p = path(CFG)
        backup(p)
        text = cfg_read(p)
        for flag, val in cfg_changes:
            key, kind, _ = CFG_KEYS[flag]
            text = cfg_set(text, key, kind, val)
            print("%s -> %s" % (key, fmt(val)))
        cfg_write(p, text)
        touched = True

    if not touched:
        show()
    elif game_running():
        print("\nthe game is running -- both files are read at launch, so restart it")


if __name__ == "__main__":
    main()
