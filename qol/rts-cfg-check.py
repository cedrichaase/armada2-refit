#!/usr/bin/env python3
"""Check that RTS_CFG.h is what QOL.asi expects: stock, at least on the line it scales.

    qol/rts-cfg-check.py [GAME]          report
    qol/rts-cfg-check.py --fix [GAME]    put FASTSCROLL_COEFFICIENT back to stock

Network games compare a CRC of RTS_CFG.h's bytes, so any edit locks a player out of
games with stock players (qol/README.md). QOL.asi scales FASTSCROLL_COEFFICIENT in
memory instead; an edited value in the file would be scaled again. --fix restores that
one line from the stock RTS_CFG.h.a2neb-backup beside it, keeping the file's CRLF, and
leaves every other line alone -- those are the player's -- but says if the file still
differs from stock.
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import a2env  # noqa: E402

KEY = "FASTSCROLL_COEFFICIENT"
STOCK_VALUE = "0.005"
LINE = re.compile(rb"^[ \t]*float[ \t]+" + KEY.encode() + rb"[ \t]*=[^\r\n]*", re.M)


def value(text):
    m = LINE.search(text)
    if not m:
        return None, None
    v = re.search(rb"=\s*([-0-9.eE+]+)", m.group(0))
    return m, (v.group(1).decode() if v else None)


def main():
    args = [a for a in sys.argv[1:] if a != "--fix"]
    fix = "--fix" in sys.argv[1:]
    game = args[0] if args else a2env.GAME
    cfg = os.path.join(game, "RTS_CFG.h")
    bak = cfg + ".a2neb-backup"
    if not os.path.exists(cfg):
        print("RTS_CFG.h: not found in %s" % game)
        return 0
    live = open(cfg, "rb").read()
    stock = open(bak, "rb").read() if os.path.exists(bak) else None

    m, v = value(live)
    if m is None:
        print("RTS_CFG.h: no %s line; the game keeps its compiled default "
              "and QOL.asi leaves it unscaled" % KEY)
        return 0
    sm, sv = value(stock) if stock is not None else (None, STOCK_VALUE)

    if v == sv and (stock is None or m.group(0) == sm.group(0)):
        print("RTS_CFG.h: %s = %s, stock" % (KEY, v))
    elif not fix:
        print("RTS_CFG.h: %s = %s, stock is %s -- QOL.asi would scale the edit, and "
              "network games refuse a changed file. Run with --fix." % (KEY, v, sv))
        return 1
    elif sm is None:
        print("RTS_CFG.h: %s = %s, stock is %s, and there is no .a2neb-backup to "
              "restore the line from -- edit it back by hand" % (KEY, v, sv))
        return 1
    else:
        live = live[:m.start()] + sm.group(0) + live[m.end():]
        with open(cfg, "wb") as fh:
            fh.write(live)
        print("RTS_CFG.h: %s %s -> %s (stock; QOL.asi scales it in memory)" % (KEY, v, sv))

    if stock is not None and live != stock:
        print("RTS_CFG.h: other lines still differ from %s -- network games will "
              "refuse this file until it matches stock" % os.path.basename(bak))
    return 0


if __name__ == "__main__":
    sys.exit(main())
