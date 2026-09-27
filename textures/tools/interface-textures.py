#!/usr/bin/env python3
"""Every texture declared under `@tmaterial=interface` in Sprites/, one per line.

These are the textures that CRASH THE GAME above 256x256 -- `@tmaterial=interface`
sprites at 512 kill the process at the cinematic-to-HUD transition, in a `rep movsd`
inside Armada2.exe. `maxsize=256` in target.conf enforces that per target, which is
enough while each texture belongs to exactly one target. It is not enough in general.

borgUI3 is why this exists. It is an interface sprite in Sprites/gui_borg.spr AND it is
referenced by Borpod16.SOD, so a census that collects "textures a SOD points at" picks it
up as a hull texture and puts it in a hull target, which has no maxsize. It was installed
at 1024 on exactly that path. A texture's target membership does not tell you what the
engine thinks it is; the sprite files do.

The .spr format is a flat list of sprite definitions under the most recent @tmaterial
line, so the material has to be tracked as state while reading.

Use -a on anything in Sprites/: the 8 font .spr files contain enough non-text that
`file` reports `data` and plain grep skips them SILENTLY -- a survey of @tmaterial values
once reported zero font materials for exactly that reason.
"""
import os, re, sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
import a2env  # noqa: E402  (the repository root, for where the game is)
GAME = a2env.GAME
SPR = os.path.join(GAME, 'Sprites')


def interface_textures():
    out = set()
    if not os.path.isdir(SPR):
        return out
    for fn in sorted(os.listdir(SPR)):
        p = os.path.join(SPR, fn)
        if not os.path.isfile(p):
            continue
        mat = None
        # latin-1 and errors=replace: these files are part binary, and decoding must not
        # throw or silently skip, which is the trap the docstring describes.
        for line in open(p, encoding='latin-1', errors='replace'):
            s = line.strip()
            m = re.match(r'@tmaterial\s*=\s*(\S+)', s)
            if m:
                mat = m.group(1).lower()
                continue
            if mat != 'interface' or not s or s.startswith('@') or s.startswith('#'):
                continue
            parts = s.split()
            # "<spritename> <texture> <x> <y> <w> <h>"
            if len(parts) >= 6 and parts[-1].lstrip('-').isdigit():
                out.add(parts[1])
    return out


if __name__ == '__main__':
    for t in sorted(interface_textures()):
        print(t)
