#!/usr/bin/env python3
"""Classify EVERY texture in the game: done, could be done, or should not be -- and why.

    textures/tools/inventory.py            a summary table
    textures/tools/inventory.py --md       INVENTORY.md, generated

This is a tool rather than a written document because a written one rots. Every figure
below is read from the game directory and the repo at the moment it runs: what is
installed is "has a .a2neb-backup beside it", what a texture is used for comes from the
SODs and the sprite files, and what it means comes from its own header and pixels.

The three verdicts:
  DONE        installed by this project
  COULD       nothing stops it; it is a scope or value decision
  SHOULD NOT  there is a reason, and the reason is recorded per category
"""
import os, re, sys, collections

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
import a2env  # noqa: E402  (the repository root, for where the game is)
GAME = a2env.GAME
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TEX, SOD, ODF, SPR = (os.path.join(GAME, *p) for p in
                      (('Textures', 'RGB'), ('SOD',), ('odf',), ('Sprites',)))

# ---- what the engine thinks each texture is -------------------------------------
def sprite_materials():
    """texture -> set of @tmaterial values it is declared under, across Sprites/.
    latin-1 with replace: these files are part binary and plain text reads skip them
    silently -- the trap that once reported zero font materials."""
    out = collections.defaultdict(set)
    if not os.path.isdir(SPR):
        return out
    for fn in sorted(os.listdir(SPR)):
        p = os.path.join(SPR, fn)
        if not os.path.isfile(p):
            continue
        mat = None
        for line in open(p, encoding='latin-1', errors='replace'):
            s = line.strip()
            m = re.match(r'@tmaterial\s*=\s*(\S+)', s)
            if m:
                mat = m.group(1).lower(); continue
            if not s or s.startswith(('@', '#')) or mat is None:
                continue
            parts = s.split()
            if len(parts) >= 6 and parts[-1].lstrip('-').isdigit():
                out[parts[1]].add(mat)
    return out


def sod_users():
    """Every place a texture name can appear, not just the SODs.

    An earlier revision of this read SOD/ and Sprites/ only, and declared 260 textures
    orphaned. Spot-checking 40 of them against odf/ and techtree/ found 6 that were
    referenced -- 15% wrong, in the direction of "there is no reason to keep this". A
    wrong orphan call is the one mistake in this file that could get real art deleted or
    silently skipped forever, so it reads everything: model files, sprite definitions,
    unit and weapon ODFs, the tech tree, and the binary maps.
    """
    out = collections.defaultdict(set)

    def add_tokens(blob, label):
        for s in {x.decode(errors='replace').lower()
                  for x in set(re.findall(rb'[ -~]{3,}', blob))}:
            out[s].add(label)
            # ODF and techtree lines are `key = "value"` or bare words; split them so a
            # texture named inside a longer line is still found.
            for tok in re.split(r'[^A-Za-z0-9_.\-]+', s):
                if len(tok) >= 3:
                    out[tok].add(label)

    for d in (SOD, ODF, os.path.join(GAME, 'techtree'), os.path.join(GAME, 'bzn')):
        if not os.path.isdir(d):
            continue
        for root, _, fns in os.walk(d):
            for fn in fns:
                try:
                    add_tokens(open(os.path.join(root, fn), 'rb').read(), fn)
                except OSError:
                    continue
    return out


def odf_kinds():
    out = {}
    for kind in ('ships', 'stations', 'other', 'weapons', 'special_weapons'):
        d = os.path.join(ODF, kind)
        if not os.path.isdir(d):
            continue
        for f in os.listdir(d):
            if f.lower().endswith('.odf'):
                out[f[:-4].lower()] = kind
    return out


def hdr(p):
    b = p + '.a2neb-backup'
    if os.path.exists(b):
        p = b
    d = open(p, 'rb').read(18)
    return dict(w=d[12] | d[13] << 8, h=d[14] | d[15] << 8, bpp=d[16], desc=d[17])


# Per-category judgement. Generated docs that only count things are not much use; the
# question is always "and should we?". Keep these honest -- several say no.
COMMENTARY = {
 'weapon texture -- beams, torpedoes, mines, shield effects':
   "**Worth doing, and the largest easy win left.** These are excluded only because the "
   "brief was ships and stations. Phaser and torpedo sprites, the Borg portal effect, "
   "`FluidicRift`, the shuttle-sized `Kbee`/`Rbee`/`Bbee` craft. Many are additive "
   "sprites, where the mean IS the light contributed, so `verify.py`'s per-channel mean "
   "check is the binding invariant rather than a nicety -- and the invention metric "
   "misreads them badly (it scores a tightened glow as erasure, see README). Judge them "
   "on a render and on the mean, not on the metric.",
 'effect texture -- flares, debris, explosions, trails':
   "**Worth doing, with care.** Flares, debris, warp trails, explosion sprites. Most are "
   "additive on black, so the same caution as the weapons applies: the channel mean is "
   "the light contributed, `verify.py` guards it, and the invention metric reads a "
   "tightened glow as erasure. Several are also tiny (32-64px), where the lift is 16x "
   "and invention is at its worst -- CLAUDE.md rule 4. Size these down, not up.",
 'map, planet or background art':
   "**Mostly already done; the remainder is deliberate.** The nebulae, skyboxes and "
   "planets were this project's original subject. What is left here is the residue: "
   "`Mbgstars` (a sparse starfield -- an upscaler turns a 1px star into a blob, and "
   "plain Lanczos buys nothing), and background art drawn too small to repay it. Check "
   "README.md before reopening any of these; several are closed with reasons.",
 'menu, loading or credits screen':
   "**Doable, genuinely low value.** Loading screens and the credits plate, shown once "
   "and often behind text. `logos` additionally carries third-party trademarks.",
 'referenced, but not by a unit':
   "**Check each one.** Referenced from somewhere real -- a map, the tech tree, a sprite "
   "-- but not by a ship or station. Set dressing and props. What draws it decides the "
   "size: a planet is drawn large and repays the work, a background prop may never "
   "exceed a few dozen pixels.",
 'hull or station texture not yet in a target':
   "**Worth doing, small.** Hull or station art a SOD points at that fell outside every "
   "faction target -- mostly because it carries a prefix the faction census did not map "
   "(map props, the neutral/other units) or because it shares a name shape with "
   "something excluded. No technical obstacle; each needs its alpha classified with "
   "`textures/tools/classify-alpha.py` first, like everything else.",
 'referenced, but not by a ship or station':
   "**Probably worth doing.** Referenced from somewhere real -- a map, the tech tree, a "
   "sprite -- but not by a unit. Planets, map props and background set dressing live "
   "here. Check what draws each one before choosing a size: a planet is drawn large and "
   "benefits, a background prop may never exceed a few dozen pixels.",
 'interface sprite below the 256 ceiling':
   "**Doable, low value, and the ceiling is a hard bar.** `@tmaterial=interface` sprites "
   "above 256x256 crash the game at the cinematic-to-HUD transition. These are under it, "
   "so they could be lifted *to* 256 -- but most are drawn at or near their native size, "
   "where upscaling only adds resampling error. The UI pass already took the ones that "
   "had magnification to fight.",
 'orphan -- no SOD, sprite, ODF or techtree references it':
   "**No.** Nothing in the game names these -- not a model, sprite, unit, weapon, tech "
   "tree entry or map. Dead art shipped in 2001. `8w472_*` is the clearest case: 19 "
   "files named after Species 8472 units, referenced by nothing, from a faction that "
   "shipped partly implemented. Upscaling them costs money and disk to change pixels "
   "nothing samples. **This is the one category where being wrong matters**, because a "
   "false orphan call means real art silently never gets done -- an earlier revision of "
   "this tool read only SOD/ and Sprites/ and was 15% wrong here.",
 'bump / normal map -- encodes vectors, not colour':
   "**No.** A bump or normal map stores surface direction per texel, not brightness. "
   "Interpolating it is defensible; a generative model inventing plausible *detail* into "
   "it invents surface that scatters light wrongly. Plain Lanczos would be safe and "
   "would add no information -- the same verdict this project reached for `Mbgstars`: "
   "safe, and buys nothing. Borg carries 29 of these, which is most of why Borg's "
   "coverage is the lowest of any faction.",
 'cursor -- deliberate pixel art, magnified by the engine':
   "**No.** Non-square, hand-placed pixels. The engine magnifies the cursor itself by a "
   "constant (`[device+0x18]`), so more texels only draw a bigger cursor at the same "
   "density; resampling buys nothing and loses the pixel art. `HUD.asi` fixes their "
   "aspect in code (`hud/README.md`).",
 'font atlas -- @tmaterial=font, sampled nearest-neighbour':
   "**No, and the engine forbids it.** `@tmaterial=font` means nearest-neighbour "
   "sampling: an 8x atlas makes the GPU pick one texel out of each 8x8 block, which is "
   "a point-sampled glyph, not a sharper one. Glyphs also ship one atlas per point size "
   "precisely because they are drawn 1:1. And unlike gas, a glyph has a right answer -- "
   "an invented stroke turns an 8 into a B.",
 'broken stock mip chain -- _1 and _2 both at half size, byte-identical':
   "**Could, carefully.** Stock ships `_1` and `_2` at the same size and byte-identical "
   "-- a copy-paste error in the original packaging, not a chain. The engine tolerates "
   "it today. Mirroring that duplication at the new scale reproduces the relationship "
   "rather than inventing one, and is probably safe; it was held back because a wrong "
   "mip chain is a hard crash rather than a blemish. Affects assimilated-station "
   "variants, visible only after a Borg capture.",
 'misnamed stock mip level -- a lone _2 at half size, with no _1':
   "**Could, carefully, and it is a different quirk from the one above.** These carry a "
   "single mip level named `_2` where the size says `_1` -- half the base, with no `_1` "
   "beside it. Mostly beam and weapon plates. Whether the engine reads a level named "
   "`_2` when `_1` is absent was not established, and that question decides the remedy: "
   "if it does, the upscale must write the new half-size level as `_2` to match; if it "
   "does not, the file is inert and the base can be upscaled with no chain at all. "
   "Answer it before building any of these -- an invalid chain is a hard crash.",
 'interface sprite already at the 256 ceiling':
   "**No.** Already at the hard ceiling that crashes the game when exceeded.",
 'colour lookup table -- interpolating it blends the cells':
   "**No.** `colors` is an 8x8 lookup table, not a picture. Interpolating between cells "
   "blends values that are meant to be discrete.",
}


# Files this project adds to Textures/RGB, which are not stock art: models/hull-bump.py.
ADDED = {'a2flatbump.tga'}


def main():
    mats, users, kinds = sprite_materials(), sod_users(), odf_kinds()
    files = [f for f in sorted(os.listdir(TEX))
             if f.endswith(('.tga', '.TGA')) and not f.endswith('.a2neb-backup')
             and f.lower() not in ADDED]
    names = {f.rsplit('.', 1)[0] for f in files}
    byname = {f.rsplit('.', 1)[0]: f for f in files}

    def width(n):
        try:
            return hdr(os.path.join(TEX, byname[n]))['w']
        except Exception:
            return 0

    def is_level(n):
        m = re.match(r'^(.+?)_?(\d)$', n)
        if not m:
            return None
        parent, lvl = m.group(1), int(m.group(2))
        if lvl >= 1 and parent in names and width(n) == width(parent) >> lvl:
            return parent
        return None

    # which target, if any, owns a texture
    owner = {}
    tdir = os.path.join(ROOT, 'targets')
    for t in sorted(os.listdir(tdir)):
        # The committed manifest, not stock/: stock/ is in A2_DATA, may not be filled
        # yet, and can only ever hold what the manifest names.
        man = os.path.join(tdir, t, 'stock.sha256')
        if not os.path.isfile(man):
            continue
        for line in open(man):
            f = line.split(None, 1)[1].strip() if len(line.split(None, 1)) == 2 else ''
            if f:
                owner[f.rsplit('.', 1)[0]] = t

    rows = []
    for n in sorted(names):
        f = byname[n]
        done = os.path.exists(os.path.join(TEX, f + '.a2neb-backup'))
        parent = is_level(n)
        mat = mats.get(n, set())
        us = users.get(n.lower(), set()) | users.get(n.lower() + '.tga', set())
        ks = {kinds.get(s.rsplit('.', 1)[0].lower()) for s in us}
        ks.discard(None)
        w = width(n)
        low = n.lower()

        if done:
            v, why = 'DONE', owner.get(n, '')
        elif parent:
            v, why = 'FOLLOWS', f'mip level of {parent}'
        elif 'font' in low or 'font' in mat:
            v, why = 'SHOULD NOT', 'font atlas -- @tmaterial=font, sampled nearest-neighbour'
        elif low.startswith('curs'):
            v, why = 'SHOULD NOT', 'cursor -- deliberate pixel art, magnified by the engine'
        elif 'bump' in low:
            v, why = 'SHOULD NOT', 'bump / normal map -- encodes vectors, not colour'
        elif not us and not mat:
            v, why = 'SHOULD NOT', 'orphan -- no SOD, sprite, ODF or techtree references it'
        elif n in ('colors',):
            v, why = 'SHOULD NOT', 'colour lookup table -- interpolating it blends the cells'
        elif 'interface' in mat and w >= 256:
            v, why = 'SHOULD NOT', 'interface sprite already at the 256 ceiling'
        elif 'interface' in mat:
            v, why = 'COULD', 'interface sprite below the 256 ceiling'
        # The game's own prefix conventions, which matter because a weapon texture is
        # referenced BY a ship's SOD and so looks like hull art to a use-based test.
        # W = weapon, X = effect, G = GUI, M = map/planet/background.
        elif re.match(r'^(LOADING|Master|PopUp|logos|gamesel|credit)', n, re.I):
            v, why = 'COULD', 'menu, loading or credits screen'
        elif n[0] in 'Ww' and not (ks & {'ships', 'stations'} and n.lower().startswith('wireframe')):
            v, why = 'COULD', 'weapon texture -- beams, torpedoes, mines, shield effects'
        elif n[0] in 'Xx':
            v, why = 'COULD', 'effect texture -- flares, debris, explosions, trails'
        elif n[0] in 'Mm':
            v, why = 'COULD', 'map, planet or background art'
        elif ks & {'weapons', 'special_weapons'}:
            v, why = 'COULD', 'weapon texture -- beams, torpedoes, mines, shield effects'
        elif ks & {'ships', 'stations'}:
            v, why = 'COULD', 'hull or station texture not yet in a target'
        else:
            v, why = 'COULD', 'referenced, but not by a unit'
        rows.append((n, v, why, w, done))

    # a broken stock chain outranks the generic reasons
    for i, (n, v, why, w, done) in enumerate(rows):
        if done or v == 'FOLLOWS':
            continue
        # Each candidate is compared against ITS OWN level width. Comparing against the
        # set {w>>1, w>>2} lets a bogus `_2` that happens to be w>>1 pass as valid --
        # which is exactly the shape stock's broken chains have (`_1` and `_2` both at
        # half size, byte-identical), so the test missed 7 of the 9 that exist.
        # Which spellings can be a stray depends on how this texture's own chain is
        # spelled -- the same rule as mip_strays() in textures/lib/common.sh. The underscore form
        # always counts; the bare form counts only where level 1 is itself bare.
        # Otherwise `fdestroy2` (the Sabre) reads as a broken level 2 of `fdestroy`
        # (the Defiant), which is a different texture that merely shares a name prefix.
        bare_chain = f'{n}1' in names and width(f'{n}1') == w >> 1
        strays = []
        for lvl in (1, 2):
            cands = [f'{n}_{lvl}'] + ([f'{n}{lvl}'] if bare_chain else [])
            for c in cands:
                if c in names and width(c) != w >> lvl:
                    strays.append(c)
        if strays:
            has1 = f'{n}_1' in names or (bare_chain and f'{n}1' in names)
            if has1:
                why2 = ('broken stock mip chain -- _1 and _2 both at half size, '
                        'byte-identical')
            else:
                why2 = ('misnamed stock mip level -- a lone _2 at half size, with no _1')
            rows[i] = (n, 'SHOULD NOT', why2, w, done)

    if '--md' in sys.argv:
        byv = collections.Counter(r[1] for r in rows)
        grp = collections.defaultdict(list)
        for n, v, why, w, done in rows:
            grp[(v, why)].append(n)
        tgt = collections.Counter()
        for n, v, why, w, done in rows:
            if v == 'DONE':
                tgt[why or '(installed before targets existed)'] += 1
        o = []
        A = o.append
        A("# Inventory: what is done, what could be, and what should not be\n")
        A("**Generated by `textures/tools/inventory.py --md`. Do not hand-edit.** Every figure is")
        A("read from the game directory and this repo at the moment it runs: *done* means")
        A("the file has a `.a2neb-backup` beside it; what a texture is *used for* comes")
        A("from the SODs, the sprite definitions, the unit and weapon ODFs, the tech tree")
        A("and the binary maps; what it *is* comes from its own header and pixels.")
        A("Re-run it rather than trusting a number below.\n")
        A(f"`Textures/RGB` holds **{len(rows)} texture files**.\n")
        A("| verdict | files | |")
        A("|---|---:|---|")
        A(f"| **done** | {byv['DONE']} | installed by this project |")
        A(f"| **follows** | {byv['FOLLOWS']} | mip levels of a skipped texture; they follow their base and are not separate decisions |")
        A(f"| **could** | {byv['COULD']} | nothing stops it — a scope or value decision |")
        A(f"| **should not** | {byv['SHOULD NOT']} | there is a reason, below |\n")
        A("Note that **done** counts files, not textures: a 1024px base and its two")
        A("rebuilt mip levels are three files. `./a2tex verify` reports textures.\n")
        A("## Done\n")
        A("By the target that owns it. `./a2tex list` shows them all; each target's")
        A("`target.conf` carries the reasoning for its own settings.\n")
        A("| target | files |")
        A("|---|---:|")
        for t, c in sorted(tgt.items(), key=lambda kv: (-kv[1], kv[0])):
            A(f"| `{t}` | {c} |")
        A("")
        for verdict, head in (('COULD', 'Could be done'),
                              ('SHOULD NOT', 'Should not be done')):
            A(f"## {head}\n")
            cats = sorted(((w, len(ns)) for (v, w), ns in grp.items() if v == verdict),
                          key=lambda kv: -kv[1])
            for why, c in cats:
                names = sorted(grp[(verdict, why)])
                A(f"### {c} — {why}\n")
                A(COMMENTARY.get(why, "") + "\n")
                shown = names[:40]
                tail = f", and {len(names) - len(shown)} more" if len(names) > len(shown) else ""
                A("`" + "`, `".join(shown) + "`" + tail + "\n")
        open(os.path.join(ROOT, 'INVENTORY.md'), 'w').write("\n".join(o) + "\n")
        print(f"INVENTORY.md: {len(o)} lines")
        return rows
    agg = collections.Counter(r[1] for r in rows)
    print(f"{len(rows)} texture files\n")
    for v in ('DONE', 'FOLLOWS', 'COULD', 'SHOULD NOT'):
        print(f"  {agg[v]:5}  {v}")
    print()
    for v in ('COULD', 'SHOULD NOT'):
        print(f"--- {v} ---")
        for why, c in collections.Counter(
                r[2] for r in rows if r[1] == v).most_common():
            print(f"  {c:5}  {why}")
        print()
    return rows


if __name__ == '__main__':
    main()
