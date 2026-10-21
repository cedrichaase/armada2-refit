#!/usr/bin/env python3
"""Rewrite vkd3d's legacy (d3dbc) shader bytecode into what Windows' own Direct3D 9
accepts. DXVK takes vkd3d's output as it is; Windows' runtime validates every shader at
CreateVertexShader/CreatePixelShader and refuses, with D3DERR_INVALIDCALL, three things
vkd3d writes freely (measured on Windows 10, render3d/README.md, "Shaders on Windows"):

  - an instruction reading two different constant registers (`mul r0.x, c0.x, c12.x`),
    or two different input registers (`mul r1.x, v0.x, v2.y`): shader model 3.0 reads
    one of each per instruction. The same one twice is accepted. Every other one is
    copied into a temporary first, only the lanes the instruction consumes, so an input
    is never read outside its dcl mask (`mov rT.x, v2.y` then `mul r1.x, v0.x, rT.x`);
  - `sincos` writing the register it reads (`sincos r1.y, r1.x`). It writes a
    temporary, and a `mov` puts that where it was meant to go;
  - `oDepth` written with a partial mask (`mov oDepth.x, r0.x`): it takes the full one.

Shader model 3.0 has 32 temporaries (r0-r31), and vkd3d never reuses one, so a long
shader can name more (the nebulae's march, r33). Then they are given out again first:
each value's live range, found by liveness over the shader's if/else (no loops: vkd3d
unrolls them), gets the lowest register free over all of it. A value live into or out of
an if keeps one register across the whole of it, so both ways through agree.

The temporary a rewrite needs is one the shader never uses, else one it never reads
again after that instruction (every instruction that can run later comes later in the
list).

Two more are refused that a rewrite cannot mend, so --check reports them and a rewrite
fails on them: a texld (or dsx, dsy) inside an `if` (the shader has to sample with an
explicit LOD or gradients there, or outside the branch); and a shader of more
instruction slots than the card takes (D3DCAPS9's MaxPixelShader30InstructionSlots,
4096 on some cards; the Radeon we measured on takes 32768). Such a shader has to be cut
into passes (src/gfx/chain.rs). --long lets one through that the code can do without
(it logs the refusal and stands that part down).

  d3dbc-fix.py [--long] <in.bin> <out.bin>  rewrite
  d3dbc-fix.py --check <file.bin>...        list what Windows would refuse; exit 1 if any
"""
import struct
import sys

TEMPS = 32
OP_MOV, OP_SINCOS, OP_DCL, OP_DEF, OP_DEFI, OP_DEFB = 1, 37, 31, 81, 49, 48
# Instructions with no destination token: if, ifc, else, endif, loop, endloop, rep,
# endrep, break, breakc, texkill, call, callnz, ret, label.
NO_DST = {40, 41, 42, 43, 27, 29, 38, 39, 44, 45, 65, 25, 26, 28, 30}
LOOPS = {27, 38, 25, 26}    # loop, rep, call, callnz: not linear any more
T_TEMP, T_INPUT, T_CONST, T_DEPTH = 0, 1, 2, 9
PORTS = {T_INPUT: 'input', T_CONST: 'constant'}     # one register of each per instruction
# Instructions whose sources are read as one lane through a replicate swizzle: rcp, rsq,
# exp, log, pow, sincos.
SCALAR = {6, 7, 14, 15, 32, 37}


def rtype(t):
    return ((t >> 28) & 7) | (((t >> 11) & 3) << 3)


def regnum(t):
    return t & 0x7ff


def parse(data):
    w = list(struct.unpack(f'<{len(data) // 4}I', data))
    ins, i = [], 1
    while i < len(w):
        t = w[i]
        if t == 0xffff:
            break
        n = (t >> 16) if (t & 0xffff) == 0xfffe else (t >> 24) & 0xf
        ins.append(w[i:i + 1 + n])
        i += 1 + n
    return w[0], ins


def emit(ver, ins):
    toks = [ver] + [x for I in ins for x in I] + [0xffff]
    return struct.pack(f'<{len(toks)}I', *toks)


def code(I):
    """(opcode, dst token or None, source tokens) of an instruction; None for a
    comment, a declaration or a definition."""
    op = I[0] & 0xffff
    if op in (0xfffe, OP_DCL, OP_DEF, OP_DEFI, OP_DEFB):
        return None
    if any((x >> 13) & 1 for x in I[1:]):
        raise ValueError('relative addressing is not handled')
    if (I[0] >> 28) & 1:
        raise ValueError('predication is not handled')
    if op in NO_DST:
        return op, None, I[1:]
    return op, I[1], I[2:]


def problems(I):
    c = code(I)
    if not c:
        return []
    op, dst, srcs = c
    out = []
    for ty, what in PORTS.items():
        if len({regnum(s) for s in srcs if rtype(s) == ty}) > 1:
            out.append(f'two {what} registers')
    if op == OP_SINCOS and rtype(dst) == T_TEMP and rtype(srcs[0]) == T_TEMP and \
            regnum(dst) == regnum(srcs[0]):
        out.append('sincos writes its source')
    if dst is not None and rtype(dst) == T_DEPTH and (dst >> 16) & 0xf != 0xf:
        out.append('oDepth with a partial mask')
    if any(rtype(t) == T_TEMP and regnum(t) >= TEMPS for t in ([dst] if dst else []) + list(srcs)):
        out.append(f'a temporary past r{TEMPS - 1}')
    return out


UNFIXABLE = ('gradient inside an if',)
GRADIENT = {66, 91, 92}     # texld, dsx, dsy: they take neighbouring pixels' values


# The fewest instruction slots a card may take for a shader model 3 shader is 512; 4096
# is what the cards we know of that take fewer than the 32768 maximum report, and a shader
# past it is refused at CreatePixelShader. Slots per instruction, as the runtime counts
# them (most take one; render3d's gfx::slots counts the same way).
SLOT_CAP = 4096
SLOT_COST = {18: 2, 20: 4, 21: 3, 22: 4, 23: 3, 24: 2, 25: 2, 26: 3, 27: 3, 29: 2, 32: 3, 33: 2, 34: 3,
             36: 3, 37: 8, 38: 3, 39: 2, 40: 3, 41: 3, 45: 3, 65: 2, 91: 2, 92: 2, 93: 3, 95: 2, 96: 3,
             0: 0, 30: 0}


def slots(ins):
    return sum(SLOT_COST.get(I[0] & 0xffff, 1) for I in ins if code(I))


def all_problems(ins):
    """[(instruction index, problem)] for a whole shader: problems() of each, and a
    gradient instruction inside an if."""
    found, depth = [], 0
    for k, I in enumerate(ins):
        op = I[0] & 0xffff
        depth += (op in (40, 41)) - (op == 43)
        found += [(k, p) for p in problems(I)]
        if op in GRADIENT and depth > 0:
            found.append((k, 'gradient inside an if'))
    return found


def lanes(op, dst, k):
    """(mask, swizzle) for a copy of source `k`: the lanes the instruction consumes of
    it, and the swizzle that reads them back from the copy."""
    if op in SCALAR or (op == 90 and k == 2):      # dp2add's third source is scalar too
        return 0x1, 0x00
    if op == 90:
        return 0x3, 0xe4
    if op == 8:                                    # dp3
        return 0x7, 0xe4
    if op in (9, 66, 93, 95) or dst is None:       # dp4, texld*, and no destination
        return 0xf, 0xe4
    return (dst >> 16) & 0xf, 0xe4


def temps_read(I):
    c = code(I)
    return {regnum(s) for s in c[2] if rtype(s) == T_TEMP} if c else set()


def temps_any(I):
    c = code(I)
    if not c:
        return set()
    regs = [c[1]] if c[1] is not None else []
    return {regnum(s) for s in regs + list(c[2]) if rtype(s) == T_TEMP}


def scratch(ins, at, avoid, count):
    """`count` temporaries free to overwrite just before instruction `at`."""
    used = set().union(*(temps_any(I) for I in ins))
    later = set().union(set(), *(temps_read(I) for I in ins[at:]))
    free = [r for r in range(TEMPS) if r not in used] + \
           [r for r in range(TEMPS) if r in used and r not in later]
    free = [r for r in free if r not in avoid]
    if len(free) < count:
        raise ValueError(f'no free temporary at instruction {at}')
    return free[:count]


def accesses(I):
    """(read, written) temporary components of an instruction, as bitsets of
    register * 4 + component."""
    c = code(I)
    if not c:
        return 0, 0
    op, dst, srcs = c
    r = w = 0
    for k, t in enumerate(srcs):
        if rtype(t) != T_TEMP:
            continue
        if op == 65:                                   # texkill: its operand is a mask
            comps = range(4)
        else:
            mask, _ = lanes(op, dst, k)
            sw = (t >> 16) & 0xff
            comps = {(sw >> (2 * l)) & 3 for l in range(4) if mask >> l & 1}
        for cc in comps:
            r |= 1 << (regnum(t) * 4 + cc)
    if dst is not None and rtype(dst) == T_TEMP:
        for cc in range(4):
            if (dst >> 16) >> cc & 1:
                w |= 1 << (regnum(dst) * 4 + cc)
    return r, w


def liveness(ins):
    """The temporary components live before each instruction (bitsets, as accesses()),
    through if/else/endif: a side's own writes end a value on that side, and the two
    sides join at the if."""
    before, live, stack = [0] * len(ins), 0, []
    for k in range(len(ins) - 1, -1, -1):
        op = ins[k][0] & 0xffff
        if op == 43:                                   # endif, met from below
            stack.append([live, None])
        elif op == 42:                                 # else: its side is done
            stack[-1][1] = live
            live = stack[-1][0]
        elif op in (40, 41):                           # if: the then side is done
            after, other = stack.pop()
            live |= after if other is None else other
        if op not in (42, 43):
            r, w = accesses(ins[k])
            live = (live & ~w) | r
        before[k] = live
    return before


def regs_of(bits):
    return {b // 4 for b in range(bits.bit_length()) if bits >> b & 1}


def allocate(ins):
    """The temporaries given out again (above): each register's uses split into the
    runs of instructions over which it is live, and each run the lowest register no
    overlapping run has."""
    before = liveness(ins)
    n = len(ins)
    use = {}                                           # register -> instructions it is in use at
    for k in range(n):
        for r in regs_of(before[k]) | regs_of(accesses(ins[k])[1]):
            use.setdefault(r, set()).add(k)
    # A register live into or out of an if is in use over all of it.
    opens = []
    for k in range(n):
        op = ins[k][0] & 0xffff
        if op in (40, 41):
            opens.append(k)
        elif op == 43:
            a = opens.pop()
            across = regs_of(before[a]) | (regs_of(before[k + 1]) if k + 1 < n else set())
            for r in across:
                use.setdefault(r, set()).update(range(a, k + 1))
    runs = []                                          # (first, last, register)
    for r, ks in use.items():
        ks = sorted(ks)
        first = prev = ks[0]
        for k in ks[1:]:
            if k != prev + 1:
                runs.append((first, prev, r))
                first = k
            prev = k
        runs.append((first, prev, r))
    runs.sort()
    active, given = [], {}                             # (last, register given), run -> register
    for first, last, r in runs:
        active = [a for a in active if a[0] >= first]
        taken = {g for _, g in active}
        g = next(x for x in range(len(runs) + 1) if x not in taken)
        given[(first, last, r)] = g
        active.append((last, g))
    if max(given.values(), default=0) >= TEMPS:
        raise ValueError(f'needs {max(given.values()) + 1} temporaries at once')
    owner = {}                                         # (instruction, register) -> register given
    for (first, last, r), g in given.items():
        for k in range(first, last + 1):
            owner[(k, r)] = g
    out = []
    for k, I in enumerate(ins):
        if code(I) is None:
            out.append(I)
            continue
        new = [I[0]]
        for t in I[1:]:
            if rtype(t) == T_TEMP:
                t = (t & ~0x7ff) | owner[(k, regnum(t))]
            new.append(t)
        out.append(new)
    return out


def mov(dst_reg, src_tok, mask=0xf, mod=0):
    return [0x02000001, 0x80000000 | (mask << 16) | (mod << 20) | dst_reg,
            src_tok]


def fix(data, long=False):
    ver, ins = parse(data)
    bad = [f'instruction {k}: {p}' for k, p in all_problems(ins) if p in UNFIXABLE]
    if bad:
        raise ValueError(f'cannot be mended here: {bad[0]} ({len(bad)} in all)')
    if any((I[0] & 0xffff) in LOOPS for I in ins):
        raise ValueError('loops or calls: "never read again" is not linear')
    past = f'a temporary past r{TEMPS - 1}'
    out = allocate(ins) if any(past in problems(I) for I in ins) else list(ins)
    i = 0
    while i < len(out):
        I = out[i]
        p = problems(I)
        if not p:
            i += 1
            continue
        op, dst, srcs = code(I)
        lead = 1 if dst is None else 2      # where the sources start in I
        if any(x.startswith('two ') for x in p):
            extra = []                         # (source index, type, register) to copy
            for ty in PORTS:
                keep = None
                for k, t in enumerate(srcs):
                    if rtype(t) != ty:
                        continue
                    if keep is None:
                        keep = regnum(t)
                    elif regnum(t) != keep:
                        extra.append(k)
            regs = scratch(out, i, temps_any(I), len(extra))
            new, pre = list(I), []
            for k, r in zip(extra, regs):
                mask, swz = lanes(op, dst, k)
                pre.append(mov(r, srcs[k], mask))
                new[lead + k] = 0x80000000 | (swz << 16) | r
            out[i:i + 1] = pre + [new]
            i += len(pre)
            continue
        if 'oDepth with a partial mask' in p:
            out[i] = [I[0], I[1] | (0xf << 16)] + list(I[2:])
            continue
        # sincos rD.m, rD.s  ->  sincos rT.m, rD.s ; mov rD.m, rT (modifiers on the mov)
        (r,) = scratch(out, i + 1, temps_any(I), 1)
        mask, mod = (dst >> 16) & 0xf, (dst >> 20) & 0xf
        sc = [I[0], (dst & ~(0xf << 20) & ~0x7ff) | r] + list(srcs)
        out[i:i + 1] = [sc, mov(regnum(dst), 0x80e40000 | r, mask, mod)]
        i += 2
    left = all_problems(out)
    if left:
        raise ValueError(f'instruction {left[0][0]} still refused: {left[0][1]}')
    n = slots(out)
    if n > SLOT_CAP and not long:
        raise ValueError(f'{n} instruction slots, past the {SLOT_CAP} some cards take: cut it into '
                         'passes, or mark its entry :long if the code stands down without it')
    return emit(ver, out)


def main(argv):
    if len(argv) >= 2 and argv[0] == '--check':
        bad = 0
        for f in argv[1:]:
            _, ins = parse(open(f, 'rb').read())
            found = all_problems(ins)
            for k, p in found:
                print(f'{f}: instruction {k}: {p}')
            n = slots(ins)
            if n > SLOT_CAP:
                print(f'{f}: {n} instruction slots, past {SLOT_CAP}')
            bad += bool(found) or n > SLOT_CAP
        return 1 if bad else 0
    long = bool(argv) and argv[0] == '--long'
    if long:
        argv = argv[1:]
    if len(argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    try:
        data = fix(open(argv[0], 'rb').read(), long)
    except ValueError as e:
        print(f'd3dbc-fix.py: {argv[0]}: {e}', file=sys.stderr)
        return 1
    open(argv[1], 'wb').write(data)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
