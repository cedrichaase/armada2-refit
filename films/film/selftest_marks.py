#!/usr/bin/env python3
"""Pins the event marks (films/README.md, "Marks") without a game: Scene.marks put on the
host's clock through its epoch lines (a tick that wraps included), the runner's and
UI.log's marks, refs, `Cut:` and `Expect:`, a take whose marks miss its Expect counted
as suspect and one filmed before marks as current, and the board check's spelling
errors. Exit 0 when all of it holds.

    python3 films/film/selftest_marks.py
"""
import json
import os
import sys
import tempfile
import time

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from film import cli, marks, storyboard as sb  # noqa: E402

failed = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        failed.append(what)


def filetime(unix):
    ft = int((unix + marks.FILETIME_UNIX) * 1e7)
    return f'0x{ft >> 32:08x} 0x{ft & 0xffffffff:08x}'


T0 = 1792000000.0        # the action's start on the host clock


def scene_text():
    """A Scene.marks as Scene.asi writes it: ticks wrap past 2^32 between the epochs."""
    base = 0xffffff00                     # 256 ms before the tick wraps
    def at(s):                            # the tick s seconds after T0
        return (base + int(s * 1000)) & 0xffffffff
    lines = [
        f'epoch {filetime(T0)} {base}',
        f'{at(-2.0)} ready objects=3',
        f'{at(-1.5)} select n=0',
        f'{at(0.4)} cmd text=select warbird bop',
        f'{at(0.45)} select n=2 names=warbird,bop',
        f'{at(1.0)} glide.start secs=2.0 yaw=40.0 pitch=-6.0 dist=300.0',
        f'{at(1.02)} camera.move eye=1.0,2.0,3.0',
        f'epoch {filetime(T0 + 10.0)} {at(10.0)}',
        f'{at(3.0)} glide.end',
        f'{at(3.1)} camera.still eye=1.0,2.0,3.0 front=0.0,0.0,1.0',
        f'{at(4.0)} cloak.start obj=warbird',
        f'{at(6.5)} cloak.done obj=warbird',
        f'{at(9.0)} decloak.start obj=warbird',
        f'{at(11.2)} decloak.done obj=warbird',
        f'{at(11.5)} fire.start obj=fed weapon=fphaser target=0x00012345',
        f'{at(12.5)} fire.end obj=fed weapon=fphaser target=0x00012345 shots=3 secs=1.0',
        f'{at(13.0)} explosion type=fireball r=52.3 at=2000.0,0.0,2100.0 near=big d=3.2',
        f'{at(13.0)} destroyed obj=big',
        'garbage line',
    ]
    return '\r\n'.join(lines) + '\r\n'


def doc():
    events = [dict(t=0.0, op='scene', arg='select warbird bop', t1=0.6),
              dict(t=0.6, op='wait', arg='4', t1=4.6),
              dict(t=4.6, op='press', arg='shift+1', t1=4.8)]
    ui = marks.ui_marks([(T0 + 2.0, 'menus: open dialog 152 "Single Player" (proc 0x4) depth 0 hwnd 0x1 800x600 @0,0'),
                         (T0 + 2.5, 'menus: shell display mode -> 1280x720x32 (2 of 2 sites)')])
    return marks.build(T0, 14.0, scene=marks.parse_scene(scene_text()), ui=ui, events=events)


def main():
    d = doc()
    ks = [m['kind'] for m in d['marks']]
    check('epoch' not in ks and 'garbage' not in ' '.join(ks), 'epoch lines and garbage are not marks')
    t = {m['kind']: m['t'] for m in d['marks']}
    check(abs(t['ready'] + 2.0) < 0.002, f'a setup mark has a negative time (ready {t["ready"]})')
    check(abs(t['glide.end'] - 3.0) < 0.002, f'across the tick wrap, on the host clock (glide.end {t["glide.end"]})')
    check(abs(t['decloak.done'] - 11.2) < 0.002, f'after the second epoch (decloak.done {t["decloak.done"]})')
    sel = [m for m in d['marks'] if m['kind'] == 'select']
    check(sel[-1]['names'] == ['warbird', 'bop'] and sel[-1]['n'] == 2, 'select: count and names')
    cmd = [m for m in d['marks'] if m['kind'] == 'cmd'][0]
    check(cmd['text'] == 'select warbird bop', 'cmd: text runs to the end of the line')
    check(d['marks'][[m['kind'] for m in d['marks']].index('camera.still')]['front'] == [0.0, 0.0, 1.0], 'vectors')
    check(t.get('menu.open') == 2.0 and 'menu.pane' not in t, 'UI.log: a dialog opening is a mark, other lines not')
    check(t.get('action.press') == 4.6 and t.get('action.press.done') == 4.8 and 'action.wait.done' not in t,
          'the runner: a line and its end (not for wait)')

    # refs
    check(len(marks.find(d, 'select')) == 1, 'a ref sees the take only (the setup select is outside it)')
    check(len(marks.find(d, 'select', in_take=False)) == 2, '... unless asked')
    check(marks.find(d, 'cloak.*:warbird')[0]['kind'] == 'cloak.start', 'kind globs and subjects')
    check(marks.find(d, 'cloak.*#-1')[0]['kind'] == 'cloak.done', '#-1 is the last')
    check(marks.find(d, 'select:bop') and not marks.find(d, 'select:fed'), 'a subject matches a list')
    check(marks.normalize('decloak done:warbird before 8s') == 'decloak.done:warbird before 8s', 'spaced kinds')
    check(marks.normalize('action step done') == 'action.step.done', 'spaced action kinds')

    # Cut
    i, n = marks.cut(d, 'from glide.end to decloak.done:warbird +0.5')
    check(abs(i - 3.0) < 1e-6 and abs(n - 8.7) < 1e-3, f'Cut: from a mark to a mark +0.5 ({i:.2f}, {n:.2f})')
    i, n = marks.cut(d, 'from glide end -1 to end pad 0.5', speed=2.0)
    check(abs(i - 1.5) < 1e-6 and abs(n - (14.0 - 1.5) / 2) < 1e-3, f'Cut: offsets, end, pad, speed ({i:.2f}, {n:.2f})')
    for bad in ('glide.end to end', 'from glide.end', 'from nothing.here to end', 'from 5 to 2'):
        try:
            marks.cut(d, bad)
            check(False, f'Cut {bad!r} is refused')
        except marks.MarkError:
            check(True, f'Cut {bad!r} is refused')

    # Expect
    def ok(text):
        return [r[1] for r in marks.verify(d, text)]
    check(ok('selected >= 2 by 1s') == [True], 'selected >= 2 by 1s')
    check(ok('selected >= 20 by 3s') == [False], 'selected >= 20 by 3s fails')
    check(ok('selected == 0 at 0.2') == [True], 'selected at a time, the setup counted')
    check(ok('decloak done before 12s; cloak.done after glide.end') == [True, True], 'before; after a mark')
    check(ok('decloak.done before 8s') == [False], 'decloak.done before 8s fails')
    check(ok('no explosion before 12; explosion:big between 12 and 14') == [True, True], 'no; between; subject')
    check(ok('fire.* count == 2; action.press count >= 1') == [True, True], 'count')
    check(ok('menu.open:single*') == [True], 'a menu mark')
    nos = marks.build(T0, 5.0, events=[dict(t=0, op='wait', arg='5', t1=5)])
    check(marks.verify(nos, 'selected >= 1; cloak.done')[0][1] is None, 'no Scene.marks: not checked, not failed')
    for bad in ('selected 3', 'cloak.done at 3', 'select count many', 'cloak.done before'):
        try:
            marks.parse_expect(bad)
            check(False, f'Expect {bad!r} is refused')
        except marks.MarkError:
            check(True, f'Expect {bad!r} is refused')

    # a take on disk: suspect, current, filmed before marks
    with tempfile.TemporaryDirectory() as tmp:
        board = os.path.join(tmp, 'storyboard.md')
        open(board, 'w').write('# Marks test\nFilm: markstest\n\n## A\n\n### m01 · Cloak\nScene: cloak\n'
                               'Variants: stock\nLength: 5\nExpect: cloak.done before 8s\n'
                               'Cut: from glide.end to decloak.done +0.5\n\n```action\nwait 14\n```\n')
        sb.a2env.DATA = tmp
        film = sb.parse(board)
        s = film.shot('m01')
        check(not sb.check(film), 'a board with Cut and Expect checks clean')
        stem = film.take(s, 'stock', 'probe')
        os.makedirs(os.path.dirname(stem))
        open(stem + '.mkv', 'w').close()
        json.dump(dict(hash=film.spec_hash(s, 'stock', 'probe')), open(stem + '.spec.json', 'w'))
        check(cli.take_check(film, s, 'stock', 'probe', ignore_code=True)[0] == 'ok',
              'a take filmed before marks is current')
        json.dump(d, open(stem + '.marks.json', 'w'))
        check(cli.take_check(film, s, 'stock', 'probe', ignore_code=True)[0] == 'ok', 'Expect met: current')
        sug = cli.cut_suggestion(film, s, 'probe')
        check(sug and sug[0] == 'In: 3   Length: 8.7', f'Cut suggests In and Length ({sug and sug[0]})')
        s.fields['Expect'] = 'cloak.done before 5s'      # Expect is not in the spec: no re-film
        st, why = cli.take_check(film, s, 'stock', 'probe', ignore_code=True)
        check(st == 'suspect' and 'cloak.done before 5s' in why, f'Expect not met: suspect ({why})')
        s.fields['Expect'] = 'cloak.done before'
        s.fields['Cut'] = 'from x'
        probs = [m for _, m in sb.check(film)]
        check(any(m.startswith('Expect:') for m in probs) and any(m.startswith('Cut:') for m in probs),
              'the board check reports a misspelt Cut and Expect')

    # following a log as it is written
    with tempfile.NamedTemporaryFile('w', delete=False) as f:
        f.write('menus: open dialog 291 "main menu" (proc 0x2), written before\r\n')
        p = f.name
    tail = marks.Tail(p, every=0.02).start()
    with open(p, 'a') as f:
        f.write('menus: open dialog 7 "Options" (proc 0x1)\r\nhalf a li')
    time.sleep(0.15)
    with open(p, 'a') as f:
        f.write('ne\r\n')
    lines = tail.stop()
    os.unlink(p)
    check([l for _, l in lines] == ['menus: open dialog 7 "Options" (proc 0x1)', 'half a line'],
          'Tail: whole lines added since it started, as they are written')

    print(f'\n{"FAILED: " + str(len(failed)) if failed else "all"} checks' + ('' if failed else ' hold'))
    return 1 if failed else 0


if __name__ == '__main__':
    sys.exit(main())
