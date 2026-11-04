#!/usr/bin/env python3
"""RefitPlayer.ini, the player's settings overlay: the bench's way to edit it. The edit is
`a2ini::edit` (crates/a2ini), the code UI.asi runs, through the host tool host.sh builds;
there is no second implementation (tools/settings/README.md, "The writer").

    overlay.py FILE WORD... [--unset S:K ...]   set (and remove) keys of FILE
    overlay.py --get FILE S:K                   print a key's value (exit 1 if absent)

A WORD is `Section:Key=value`, the section spelled as in the overlay
(`Render3D.Modules:sky=0`, `UI.HUD:BarHidden=1`). The file is written whole to
`FILE.tmp` and renamed over FILE, as UI.asi writes it; when nothing changes it is
not written at all.
"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def a2ini():
    """The path of the built host tool (built on first use)."""
    r = subprocess.run([os.path.join(HERE, 'host.sh')], stdout=subprocess.PIPE, text=True)
    if r.returncode != 0:
        raise SystemExit('overlay.py: tools/settings/host.sh could not build a2ini')
    return r.stdout.strip()


def main(argv):
    if len(argv) == 3 and argv[0] == '--get':
        return subprocess.run([a2ini(), 'get', argv[1], argv[2]]).returncode
    if not argv or argv[0].startswith('-'):
        print(__doc__.strip(), file=sys.stderr)
        return 2
    return subprocess.run([a2ini(), 'edit'] + argv).returncode


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
