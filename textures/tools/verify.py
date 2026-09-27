#!/usr/bin/env python3
"""Check every built texture against the stock file it replaces -- FROM THE RAW BYTES.

    textures/tools/verify.py [target...]        default: every target with an out/

Why raw bytes and not ImageMagick: alpha. ImageMagick has opinions about associated vs
unassociated alpha, and those opinions are exactly what corrupted Mmoon -- a compose
onto an alpha-bearing image left its RGB premultiplied-then-divided, mean 211 against
stock's 43, and it rendered in game as a white box around the sun. A checker built on
the same library that has the opinion cannot reliably see the damage. The engine reads
bytes; so does this.

The other half of why this exists: Mmoon WAS verified, and passed, and then the build
changed (blackedge was added) and only the newly-added property was re-checked. One
command that runs the whole invariant set is the fix for that, not more discipline.

Checks, per file:
  header      idlen / colour-map / image-type / bpp / descriptor identical to stock
  size        a power of two, and a whole multiple of stock
  maxsize     honours maxsize= in target.conf
  channels    raw mean R, G, B and A within 2 of stock's
  mips        if mips=N, exactly N levels each half the previous, each one
              named and headered like ITS OWN stock level
  installed   the file in the game directory is byte-identical to out/
"""
import os, re, sys, glob

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
import a2env  # noqa: E402  (the repository root, for where the game is)
GAME = a2env.GAME
TEX = os.path.join(GAME, 'Textures', 'RGB')
DATA = os.path.join(a2env.DATA, 'textures')   # each target's stock/ and out/, see a2tex


def header(d):
    return dict(idlen=d[0], cmap=d[1], typ=d[2], w=d[12] | d[13] << 8,
                h=d[14] | d[15] << 8, bpp=d[16], desc=d[17])


def read(path):
    """Exact per-channel means. Strided slicing, not sampling: a sampled mean was
    off by up to 3 on high-variance icons, which is the same magnitude as a real
    defect and would have made the tolerance meaningless."""
    d = open(path, 'rb').read()
    hd = header(d)
    n = hd['bpp'] // 8
    need = hd['w'] * hd['h'] * n
    px = d[18 + hd['idlen']:18 + hd['idlen'] + need]
    if len(px) < need:
        hd['error'] = f"truncated: {len(px)} bytes of pixel data, need {need}"
        return hd
    c = hd['w'] * hd['h']
    hd.update(b=sum(px[0::n]) / c, g=sum(px[1::n]) / c, r=sum(px[2::n]) / c,
              a=(sum(px[3::n]) / c if n == 4 else None))
    return hd


def conf(t, key, default):
    p = os.path.join(ROOT, 'targets', t, 'target.conf')
    for line in open(p, encoding='utf-8', errors='replace') if os.path.exists(p) else []:
        if line.startswith(key + '='):
            return line.split('=', 1)[1].strip()
    return default


def stock_for(t, base):
    for ext in ('.tga', '.TGA'):
        p = os.path.join(DATA, t, 'stock', base + ext)
        if os.path.exists(p):
            return p
    return None


def installed_for(base):
    for ext in ('.tga', '.TGA'):
        p = os.path.join(TEX, base + ext)
        if os.path.exists(p):
            return p
    return None


def mip_parent(base, widths):
    """(parent, level) if `base` names a mip level of another output in this target.

    Both spellings, because stock uses both: Fbattle_1 under Fbattle, but fcruise1_B1
    under fcruise1_B and fresearch1 under fresearch. The test this replaced was
    `'_' in base and base.rsplit('_', 1)[-1].isdigit()`, which counted those two chains
    as textures in their own right -- so their levels were compared against a stock
    BASE that is twice their size, and never checked as a chain at all.

    `widths` maps base name -> width, and the WIDTH is what decides it, exactly as in
    bash `mip_name`. Matching on the name alone is not close enough: fdestroy2 is the
    Sabre Class, a texture in its own right, and a name-only rule reads it as "level 2
    of fdestroy" because fdestroy (the Defiant) is a real base in the same target. That
    is not a false alarm -- it is a SILENT SKIP. fdestroy2 fell out of the checked set
    entirely: no header check, no power-of-two check, no channel means, no chain check,
    and no failure either, just 12 textures reported where 13 were built. Level 2 of a
    1024px fdestroy would be 256px; fdestroy2 is 1024, so the size test rejects it.

    A single trailing digit, deliberately: fedpod10 is a texture, not level 0 of
    fedpod1, and levels in this game never reach 10.
    """
    m = re.match(r'^(.+?)_?(\d)$', base)
    if not m:
        return None, 0
    parent, lvl = m.group(1), int(m.group(2))
    if lvl >= 1 and parent in widths and widths.get(base) == widths[parent] >> lvl:
        return parent, lvl
    return None, 0


def stock_mip(t, base_stock, parent, lvl):
    """The stock file for level `lvl`, by the same width test mip_name uses in bash.

    Looked for beside the target's stock base first, then in the game directory, so a
    target whose stock/ carries its chain is self-describing and one whose stock/
    predates that still resolves. Returns None when neither has it.
    """
    want = read(base_stock)['w'] >> lvl
    # The game directory stops being a source of stock the moment a target is
    # installed -- TEX/Fbattle_1.tga is then this project's own 512px level, not the
    # 128px one stock shipped. Its .a2neb-backup beside it still is stock, so that is
    # tried first; without it the per-level header check silently stops running for
    # exactly the targets that are already live.
    for d in (os.path.dirname(base_stock), TEX):
        for cand in (f'{parent}_{lvl}', f'{parent}{lvl}'):
            for ext in ('.tga', '.TGA'):
                for suffix in ('.a2neb-backup', ''):
                    p = os.path.join(d, cand + ext + suffix)
                    if os.path.exists(p) and read(p)['w'] == want:
                        return p
    return None


def main():
    targets = sys.argv[1:] or sorted(
        d for d in os.listdir(os.path.join(ROOT, 'targets'))
        if glob.glob(os.path.join(DATA, d, 'out', '*.tga')))
    total = fails = 0
    for t in targets:
        outs = sorted(glob.glob(os.path.join(DATA, t, 'out', '*.tga')))
        if not outs:
            continue
        ms = int(conf(t, 'maxsize', '0'))
        mips = conf(t, 'mips', '0')          # a count, or 'auto' (per-texture)
        bad = []
        widths = {os.path.basename(f)[:-4]: read(f)['w'] for f in outs}
        for f in outs:
            base = os.path.basename(f)[:-4]
            # a mip level is checked as part of its base, not on its own
            if mip_parent(base, widths)[0]:
                continue
            total += 1
            s = stock_for(t, base)
            if not s:
                bad.append(f"{base}: no stock/ counterpart"); continue
            B, S = read(f), read(s)
            if 'error' in B:
                bad.append(f"{base}: {B['error']}"); continue
            for k in ('idlen', 'cmap', 'typ', 'bpp', 'desc'):
                if B[k] != S[k]:
                    bad.append(f"{base}: header {k} = {B[k]}, stock has {S[k]}")
            if B['w'] & (B['w'] - 1) or B['h'] & (B['h'] - 1):
                bad.append(f"{base}: {B['w']}x{B['h']} is not a power of two")
            if B['w'] % S['w'] or B['h'] % S['h']:
                bad.append(f"{base}: {B['w']}x{B['h']} is not a whole multiple of stock "
                           f"{S['w']}x{S['h']}")
            if ms and B['w'] > ms:
                bad.append(f"{base}: {B['w']}px exceeds maxsize={ms}")
            # fit=none: the tile is not a picture of its namesake (see fit() in
            # textures/lib/common.sh), so only alpha is comparable. LOADING's is a flat 255.
            for ch in ('a' if conf(t, 'fit', 'match') == 'none' else 'rgba'):
                bv, sv = B[ch], S[ch]
                if bv is None or sv is None:
                    continue
                if abs(bv - sv) > 1.0:
                    bad.append(f"{base}: raw {ch.upper()} mean {bv:.2f}, "
                               f"stock {sv:.2f}")
            # mips=auto: the expected depth is whatever stock ships for THIS texture,
            # derived the same way gen_mips derives it, so the two cannot drift apart.
            if mips == 'auto':
                want_levels = 0
                while want_levels < 12 and stock_mip(t, s, base, want_levels + 1):
                    want_levels += 1
            else:
                want_levels = int(mips)
            if want_levels:
                want, lvl = B['w'], 1
                while lvl <= want_levels:
                    want //= 2
                    msf = stock_mip(t, s, base, lvl)
                    # The out/ level must carry the name stock uses, because the engine
                    # looks a chain up by name: a level written fcruise1_B_1 is a file
                    # nothing reads, and it leaves stock's 128px fcruise1_B1 under the
                    # new base -- the invalid chain, reached while believing otherwise.
                    name = (re.sub(r'\.(tga|TGA)(\.a2neb-backup)?$', '',
                                   os.path.basename(msf))
                            if msf else f'{base}_{lvl}')
                    m = os.path.join(DATA, t, 'out', f'{name}.tga')
                    if not os.path.exists(m):
                        bad.append(f"{base}: mip level {lvl} missing (wanted {name}.tga)")
                        break
                    M = read(m)
                    if M['w'] != want:
                        bad.append(f"{name}: {M['w']}px, chain wants {want}"); break
                    # Headers per LEVEL, not inherited from the base. fresearch is a
                    # 32-bit base (desc 0x08) whose two levels are 24-bit (desc 0x00),
                    # so taking depth from the base writes a chain that disagrees with
                    # the one stock shipped -- and nothing here used to look.
                    if msf:
                        S2 = read(msf)
                        for k in ('idlen', 'cmap', 'typ', 'bpp', 'desc'):
                            if M[k] != S2[k]:
                                bad.append(f"{name}: header {k} = {M[k]}, "
                                           f"stock has {S2[k]}")
                    lvl += 1
            if conf(t, 'install', 'yes') == 'no':
                continue                      # deliberately not shipped; see a2tex
            inst = installed_for(base)
            if inst is None:
                bad.append(f"{base}: not present in the game directory")
            elif open(inst, 'rb').read() != open(f, 'rb').read():
                bad.append(f"{base}: installed file differs from out/")
        fails += len(bad)
        mark = 'ok' if not bad else f'{len(bad)} PROBLEM(S)'
        print(f"{t:14} {len(outs):4} files  {mark}")
        for b in bad:
            print(f"                    !! {b}")
    print(f"\n{total} textures checked, {fails} problem(s)")
    sys.exit(1 if fails else 0)


if __name__ == '__main__':
    main()
