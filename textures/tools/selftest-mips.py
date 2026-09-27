#!/usr/bin/env python3
"""Regression guard for mip-chain NAME resolution, in both implementations.

This exists because the failure it covers is not a bad-looking texture, it is a hard
crash: installing a bigger base while stock's levels stay behind left Armada II with an
inconsistent chain and it went down in the Klingon campaign. The guard in `a2tex
install` is what stands in the way, and until this was written that guard globbed
<base>_[0-9]* -- which does not match fcruise1_B1 or fresearch1, two real hand-authored
chains in the Federation set that are spelled without the underscore.

Both the bash (`mip_name`) and the python (`stock_mip`) resolver are checked, because
build/install use one and verify uses the other, and a disagreement between them is
precisely how a chain gets written under a name nothing reads.

    textures/tools/selftest-mips.py        exits non-zero on any failure
"""
import os, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, 'tools'))
import importlib
V = importlib.import_module('verify')
TEX = V.TEX


def tex(name):
    for ext in ('.tga', '.TGA'):
        p = os.path.join(TEX, name + ext)
        if os.path.exists(p):
            return p
    raise SystemExit(f"selftest: {name} is not in {TEX}")


def bash_mip_name(stock, lvl, mode='name'):
    out = subprocess.run(
        ['bash', '-c', f'ROOT={ROOT!r}; . "$ROOT/lib/common.sh"; mip_name "$1" "$2" "$3"',
         '_', stock, str(lvl), mode],
        capture_output=True, text=True)
    return out.stdout.strip()


fails = []


def check(what, got, want):
    if got != want:
        fails.append(f"{what}: got {got!r}, want {want!r}")


# --- the two underscore-less chains, confirmed by RMSE against a box downsample of
# --- the base (0.022/0.026 and 0.033/0.054, inside the 0.017-0.067 band the known-good
# --- underscore chains occupy). These are the cases the old glob could not see.
check('bash fcruise1_B lvl1', bash_mip_name(tex('fcruise1_B'), 1), 'fcruise1_B1')
check('bash fcruise1_B lvl2', bash_mip_name(tex('fcruise1_B'), 2), 'fcruise1_B2')
check('bash fresearch lvl1', bash_mip_name(tex('fresearch'), 1), 'fresearch1')
check('bash fresearch lvl2', bash_mip_name(tex('fresearch'), 2), 'fresearch2')

# --- the ordinary spelling still resolves
check('bash fcruise1 lvl1', bash_mip_name(tex('fcruise1'), 1), 'fcruise1_1')
check('bash fdestroy lvl2', bash_mip_name(tex('fdestroy'), 2), 'fdestroy_2')

# --- and the chain ENDS where stock's does, rather than running on
check('bash fcruise1 lvl3', bash_mip_name(tex('fcruise1'), 3), '')

# --- FluidicRift2 is 128 beside a 256 FluidicRift: level 2 by name, level 1 by size,
# --- so it is not a mip of it at all. The width test is what rejects it, and a
# --- name-only regex would take it and break a texture that was never a chain.
check('bash FluidicRift lvl2 (not a mip)', bash_mip_name(tex('FluidicRift'), 2), '')
check('bash FluidicRift lvl1 (not a mip)', bash_mip_name(tex('FluidicRift'), 1), '')

# --- python side: mip_parent must fold both spellings back onto their base
# mip_parent decides by WIDTH, not by name. A width map stands in for one target's out/.
w = {'Fbattle': 1024, 'Fbattle_1': 512, 'Fbattle_2': 256,
     'fcruise1_B': 1024, 'fcruise1_B1': 512, 'fcruise1_B2': 256,
     'fresearch': 1024, 'fresearch1': 512, 'fresearch2': 256,
     'fedpod1': 512, 'fedpod10': 512,
     # the Defiant and the Sabre: two textures, same size, and the second one's name is
     # the first one's plus a digit
     'fdestroy': 1024, 'fdestroy_1': 512, 'fdestroy_2': 256,
     'fdestroy2': 1024, 'fdestroy2_1': 512, 'fdestroy2_2': 256,
     # real UI names that a name-only rule folds away
     'commMenu1': 256, 'commMenu11': 256, 'gbbresear': 256, 'gbbresear2': 256,
     'Fyard': 1024}
check('py Fbattle_1', V.mip_parent('Fbattle_1', w), ('Fbattle', 1))
check('py fcruise1_B1', V.mip_parent('fcruise1_B1', w), ('fcruise1_B', 1))
check('py fresearch2', V.mip_parent('fresearch2', w), ('fresearch', 2))
# fedpod10 is a texture in its own right, not level 0 of fedpod1
check('py fedpod10', V.mip_parent('fedpod10', w), (None, 0))
check('py unrelated', V.mip_parent('Fyard', w), (None, 0))

# The SILENT SKIP this guards. fdestroy2 (Sabre) is not level 2 of fdestroy (Defiant):
# level 2 of a 1024 base is 256, and fdestroy2 is 1024. Before the width test it was
# dropped from the checked set entirely -- no header check, no channel means, no chain
# check, and no failure either, just a count that was quietly one short. Across the
# whole repo the name-only rule skipped 43 textures, 42 of them in the UI targets.
check('py fdestroy2 is NOT a level', V.mip_parent('fdestroy2', w), (None, 0))
check('py fdestroy2_1 IS a level', V.mip_parent('fdestroy2_1', w), ('fdestroy2', 1))
check('py commMenu11 is NOT a level', V.mip_parent('commMenu11', w), (None, 0))
check('py gbbresear2 is NOT a level', V.mip_parent('gbbresear2', w), (None, 0))

# --- and the two resolvers must AGREE on the stock file they pick
for name, lvl in (('fcruise1_B', 1), ('fresearch', 1), ('fcruise1', 2)):
    b = tex(name)
    py = V.stock_mip(None, b, name, lvl)
    sh = bash_mip_name(b, lvl, 'path')
    check(f'agree {name} lvl{lvl}',
          os.path.basename(py or ''), os.path.basename(sh or ''))

# --- fresearch changes DEPTH mid-chain: a 32-bit base over two 24-bit levels. Taking
# --- the header from the base writes a chain stock never shipped.
base_bpp = V.read(tex('fresearch'))['bpp']
lvl_bpp = V.read(V.stock_mip(None, tex('fresearch'), 'fresearch', 1))['bpp']
check('fresearch base bpp', base_bpp, 32)
check('fresearch lvl1 bpp', lvl_bpp, 24)

# --- siblings NAMED like levels that are not levels of it at any size. Stock ships a
# --- few, and each pair is byte-identical to itself, which is what a placeholder looks
# --- like rather than a chain. They must be surfaced, not silently skipped: upscaling
# --- the base would leave a stale 128px file beside a 1024px one.
def bash_strays(stock, levels):
    out = subprocess.run(
        ['bash', '-c',
         f'ROOT={ROOT!r}; . "$ROOT/lib/common.sh"; cd "$(dirname "$1")"; mip_strays "$1" "$2"',
         '_', stock, str(levels)],
        capture_output=True, text=True)
    return sorted(out.stdout.split())


def levels_of(stock):
    n = 0
    while n < 12 and bash_mip_name(stock, n + 1):
        n += 1
    return n


check('strays FpremNew_B', bash_strays(tex('FpremNew_B'), levels_of(tex('FpremNew_B'))),
      ['FpremNew_B_2'])
check('strays Fsensor_B', bash_strays(tex('Fsensor_B'), levels_of(tex('Fsensor_B'))),
      ['Fsensor_B_1', 'Fsensor_B_2'])
# fdestroy2 is the Sabre Class, a texture in its own right, NOT a stray level of
# fdestroy (the Defiant). A guard that flags it cries wolf on an ordinary target, so the
# bare <base><digit> spelling only counts where the texture's own level 1 is bare.
check('strays fdestroy (none)', bash_strays(tex('fdestroy'), levels_of(tex('fdestroy'))), [])
check('strays fcruise1_B (none)', bash_strays(tex('fcruise1_B'), levels_of(tex('fcruise1_B'))), [])
check('strays Fbattle (none)', bash_strays(tex('Fbattle'), levels_of(tex('Fbattle'))), [])

if fails:
    print('selftest-mips: FAILED')
    for f in fails:
        print('  !!', f)
    sys.exit(1)
print(f'selftest-mips: ok ({len(fails)} failures)')
