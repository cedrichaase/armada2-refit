#!/usr/bin/env python3
"""tools/layers.py on the host: --resolve's requirements and conflicts, the plan
./install runs, and a2mod's filing, on a registry made up here and on layers.toml.
Exits 1 on the first disagreement. CI runs it beside `tools/layers.py --check`."""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import layers as L  # noqa: E402

FAIL = []


def check(ok, what):
    print(('ok    ' if ok else 'FAIL  ') + what)
    if not ok:
        FAIL.append(what)


def refused(reg, **kw):
    try:
        L.resolve(reg, **kw)
    except L.Refused as e:
        return str(e)
    return None


TOY = """
schema = 1
[release]
steps = ['a']
own = ['a']
extra = []

[[layer]]
id = 'base'
install = 'base/install'
remove = 'base/install --remove'
select = 'core'
requires = []

[[layer]]
id = 'lib'
install = 'lib/install'
remove = 'lib/install --remove'
select = 'off'
requires = []

[[layer]]
id = 'app'
install = 'app/install'
remove = 'app/install --remove'
select = 'on'
requires = ['lib']

[[layer]]
id = 'twin'
install = 'app/install'
remove = 'app/install --remove'
select = 'on'
requires = []

[[layer]]
id = 'tool'
install = ['tool/a', 'tool/b']
remove = ['tool/b --remove', 'tool/a --remove']
select = 'on'
requires = []

[[layer]]
id = 'gone'
select = 'retired'
requires = []
"""

with tempfile.NamedTemporaryFile('w', suffix='.toml', delete=False) as fh:
    fh.write(TOY)
toy = L.load(fh.name)
os.unlink(fh.name)
for lay in toy['layer']:
    for k in ('files', 'replaces', 'backups'):
        lay.setdefault(k, [])

# The default: core and on, with nothing to resolve (app's lib is off, so it is a
# conflict the default has to show, not hide).
check(refused(toy) == 'app requires lib, which --without takes out',
      'default: an on layer requiring an off one is refused')
check(L.resolve(toy, only=['app']) == ['base', 'lib', 'app'],
      '--only app adds what it requires (lib) and the core layer, in registry order')
check(L.resolve(toy, only=[]) == ['base'], '--only with nothing: the core layers alone')
check(L.resolve(toy, only=['twin', 'tool']) == ['base', 'twin', 'tool'], '--only twin,tool')
check(refused(toy, only=['app'], without=['lib']) == 'app requires lib, which --without takes out',
      '--only app --without lib: a conflict, refused')
check(refused(toy, without=['base']) == 'base: a core layer, always installed',
      '--without a core layer: refused')
check(refused(toy, only=['gone']) == 'gone: a retired layer, not offered', '--only a retired layer: refused')
check((refused(toy, only=['nope']) or '').startswith('nope: no such layer'), '--only an unknown id: refused')
check(L.resolve(toy, without=['app']) == ['base', 'twin', 'tool'], '--without app (and so no conflict)')

plan = L.commands(toy, ['base', 'tool'])
check(plan == [('remove', 'lib', 'lib/install --remove'), ('remove', 'app', 'app/install --remove'),
               ('install', 'base', 'base/install'), ('install', 'tool', 'tool/a'),
               ('install', 'tool', 'tool/b')],
      'plan: removes of the offered layers left out (a shared remove once), then installs in order')
plan = L.commands(toy, ['base', 'twin'])
check(('remove', 'app', 'app/install --remove') not in plan and
      [c for v, _, c in plan if v == 'install'] == ['base/install', 'app/install'],
      'plan: a remove a selected layer shares is not run')
check(L.commands(toy, ['base', 'lib', 'app', 'twin']) ==
      [('remove', 'tool', 'tool/b --remove'), ('remove', 'tool', 'tool/a --remove'),
       ('install', 'base', 'base/install'), ('install', 'lib', 'lib/install'),
       ('install', 'app', 'app/install')],
      'plan: one install for two layers that share it, removes in the order given')

# The real registry: the default is ./install's, and a2mod's filing.
reg = L.load()
check(L.resolve(reg) == ['platform', 'display', 'qol', 'rules', 'online', 'textures',
                         'models', 'cutscenes', 'render3d', 'ui'],
      'layers.toml: the default is every core and on layer')
check(L.resolve(reg, only=['render3d']) == ['platform', 'display', 'render3d'],
      'layers.toml: --only render3d is the core layers and render3d')
check(L.resolve(reg, only=['display']) == ['platform', 'display'],
      'layers.toml: --only display names a core layer, which is in anyway')
for rel, suffix, want in [('Textures/RGB/mnebula4.TGA', L.BACKUP, 'textures'),
                          ('Textures/RGB/curs_arrow.tga', L.BACKUP, 'hud'),
                          ('Sprites/FontFinal4_12.spr', '.a2font-backup', 'hud'),
                          ('d3d8.dll', '.a2display-backup', 'display'),
                          ('Sprites/cursor.spr', L.BACKUP, None),
                          ('QOLRules.ini', None, 'rules'), ('QOL.ini', None, 'qol'),
                          ('Online.asi', None, 'online'), ('menus/x.bmp', None, 'ui'), ('Menus.asi', None, 'menus'),
                          ('MenuScale/x.bmp', None, 'menuscale'), ('Lighting.asi', None, 'parts'),
                          ('animations/Intro.mp4', None, 'cutscenes'), ('Render3D.log', None, None)]:
    got = L.classify(reg, rel, suffix)
    check(got == want, f'classify {rel} ({suffix or "added"}): {want}' + ('' if got == want else f', got {got}'))

r = subprocess.run([sys.executable, os.path.join(HERE, 'layers.py'), '--resolve', '--only', 'hud'],
                   capture_output=True, text=True)
check(r.returncode == 2 and 'refused' in r.stderr, 'the command line: a refusal exits 2')

print(f'\n{len(FAIL)} failed' if FAIL else '\nall passed')
sys.exit(1 if FAIL else 0)
