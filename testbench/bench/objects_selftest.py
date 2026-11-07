#!/usr/bin/env python3
"""The object steps' phrasings and filters (bench/objects.py), against a made-up list:
no game. `python3 testbench/bench/objects_selftest.py`."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from bench import objects as O, scenario as S   # noqa: E402

OBJS = [
    {'id': 5, 'name': 'mine', 'label': 'fgalaxy1', 'odf': 'fgalaxy', 'class': 'Galaxy Class', 'team': 1, 'mine': True,
     'kind': 'ship', 'pos': [3000, 0, 3050], 'hull': [100, 200], 'shields': [50, 50], 'selected': True, 'group': -1},
    {'id': 6, 'name': None, 'label': 'kbase1', 'odf': 'kbase', 'class': 'Starbase', 'team': 2, 'mine': False,
     'kind': 'station', 'pos': [0, 0, 0], 'hull': [1, 1], 'selected': False, 'group': 2},
    {'id': 7, 'name': None, 'label': 'pb_clssm1', 'odf': 'pb_clssm', 'class': 'Class M Planet', 'team': 0,
     'mine': False, 'kind': 'planet', 'pos': [1500, 0, 1500], 'selected': False, 'group': -1},
]

# (step, the function it must reach, the ids its filter picks, or None to skip that check)
CASES = [
    ('Expect an object of class "Galaxy" owned by player 1', 's_expect_objects', [5]),
    ('Expect no ship on team 2', 's_expect_objects', []),
    ("Expect a station not owned by the player", 's_expect_objects', [6]),
    ("Expect 1 ship of the player's", 's_expect_objects', [5]),
    ('Expect at least 1 ship within 100 of 3000,0,3000', 's_expect_objects', [5]),
    ('Expect a planet with odf "pb_*"', 's_expect_objects', [7]),
    ('Expect an object named "kbase1"', 's_expect_objects', [6]),
    ('Expect the ship named "mine" is selected', 's_expect_objects', [5]),
    ('Wait up to 60 seconds until a ship named "mine" is within 100 of 3000,0,3000', 's_wait_objects', [5]),
    ('Wait until no ship is on team 2', 's_wait_objects', []),
    ('Select the ships of class "Galaxy" on team 1', 's_select_objects', [5]),
    ('Add the station with id 6 to the selection', 's_select_objects', [6]),
    ('Center the camera on the station of class "Starbase"', 's_camera_object', [6]),
    ("Order the ships of the player's to move to 3000,0,3000", 's_order_move', [5]),
    ('Order the ship named "mine" to attack the station on team 2', 's_order_attack', [5]),
    ('Expect no crash', 's_no_crash', None),
    ('Expect the game to have exited', 's_expect_exited', None),
]


def main():
    bad = 0
    for text, want, ids in CASES:
        t = S.normalise(text)
        hit = next(((f, rx.match(t)) for rx, f in S.STEPS if rx.match(t)), (None, None))
        fn, m = hit
        ok = fn is not None and fn.__name__ == want
        got = None
        if ok and ids is not None:
            g = m.groupdict()
            noun = g.get('noun') or g.get('noun2')
            cl = g.get('clauses') if g.get('noun') else g.get('clauses2')
            flt = O.from_phrase(noun, cl or '')
            got = [o['id'] for o in OBJS if flt(o)]
            ok = got == ids
        print(('ok   ' if ok else 'FAIL ') + text + ('' if ok else f'  -> {fn and fn.__name__} {got}'))
        bad += not ok
    f = O.from_words(['ships', 'mine', 'near=3000,0,3000,100'])
    ok = [o['id'] for o in OBJS if f(o)] == [5]
    print(('ok   ' if ok else 'FAIL ') + 'cli words: ships mine near=...')
    bad += not ok
    q, _ = O.quantity('at most 1')
    bad += not (q(1) and not q(2))
    print(f'{len(CASES) + 2 - bad} passed, {bad} failed')
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main())
