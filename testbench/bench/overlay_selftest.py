#!/usr/bin/env python3
"""The bench's side of RefitPlayer.ini: a scenario's `Overlay:` header, the clone's overlay
emptied at start, `drive overlay` against a fake running game.  python3 bench/overlay_selftest.py"""
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import scenario, session  # noqa: E402

failed = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        failed.append(what)


class Log:
    def __init__(self):
        self.lines = []

    def action(self, text, status='ok', detail=None, shots=()):
        self.lines.append(text)


def fake(tmp):
    s = session.Session.__new__(session.Session)
    g = Path(tmp) / 'game'
    g.mkdir()
    Session = session.Session
    Session.game_dir = property(lambda self: g)
    s.log = Log()
    s.s = {}
    s.dir = Path(tmp) / 'run'
    return s, g


with tempfile.TemporaryDirectory() as tmp:
    md = Path(tmp) / 'x.md'
    md.write_text('# X\n\nOverlay: Render3D.Modules:sky=0 UI.HUD:Minimap=0\nResolution: 16:9\n\n1. Launch the game.\n')
    scn = scenario.parse(md)
    check(scn.overlay == 'Render3D.Modules:sky=0 UI.HUD:Minimap=0', 'Overlay: header parsed')
    md.write_text('# X\n\n1. Launch the game.\n')
    check(scenario.parse(md).overlay == '', 'no header, no overlay')

    s, g = fake(tmp)
    for n in ('RefitPlayer.ini', 'RefitPlayer.ini.tmp', 'UIPlayer.ini', 'UIPlayer.ini.migrated'):
        (g / n).write_text('[a]\nb=1\n')
    s.empty_overlay()
    check(not any((g / n).exists() for n in session.Session.OVERLAY_FILES), 'empty_overlay removes all four')
    check(s.log.lines[-1].startswith('overlay emptied'), 'empty_overlay logs "overlay emptied"')
    import inspect
    src = inspect.getsource(session.Session.launch)
    check("'RefitPlayer.ini'" not in src and 'UIPlayer' not in src,
          'launch() does not delete the overlay it was given (ui-settings-live regression)')
    check("'RefitPlayer.ini'" in inspect.getsource(session.Session.collect_logs), 'collect_logs keeps RefitPlayer.ini')
    s.set_overlay(['Render3D.Modules:sky=0', 'UI.HUD:Minimap=0'])
    txt = (g / 'RefitPlayer.ini').read_text()
    check('[Render3D.Modules]' in txt and 'sky=0' in txt and 'Minimap=0' in txt, 'set_overlay writes both keys')
    s.set_overlay(['--unset', 'UI.HUD:Minimap'])
    check('Minimap' not in (g / 'RefitPlayer.ini').read_text(), '--unset removes a key')
    try:
        s.set_overlay(['nonsense'])
        check(False, 'a malformed word is refused')
    except session.GameError:
        check(True, 'a malformed word is refused')

    # drive overlay: no UI.log -> clear failure; with a log that gains the line -> ok
    s.running = lambda: True
    session.Session.log_dirs = property(lambda self: [g])
    ok, text = s.drive_overlay(['Render3D.World:Surfaces=0'])
    check(not ok and 'UI' in text, 'drive overlay without UI.log fails clearly: ' + text[:60])
    (g / 'UI.log').write_text('start\n')
    ok, text = s.drive_overlay(['Render3D.World:Surfaces=1'])
    check(not ok and 'within 5 s' in text, 'drive overlay with a silent UI.log times out')
    import threading
    def later():
        time.sleep(0.6)
        with open(g / 'UI.log', 'a') as f:
            f.write('settings: RefitPlayer.ini changed; told Render3D\n')
    threading.Thread(target=later).start()
    ok, text = s.drive_overlay(['Render3D.World:Surfaces=0'])
    check(ok and 'changed' in text, 'drive overlay sees the hub line')

print('\n%d failed' % len(failed) if failed else '\nall passed')
sys.exit(1 if failed else 0)
