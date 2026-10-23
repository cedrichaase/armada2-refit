#!/usr/bin/env python3
"""Put each cbuffer of a preprocessed shader source in a register space of its own, for
DXIL (hlsl-rs.sh).

    dxil-spaces.py <pre.hlsl> <out.hlsl>

Every cbuffer of a stage sits at one register, b0 for a vertex shader and b1 for a
pixel shader, and its members at packoffset(cN): together they are Direct3D 9's
register file of that stage (gpu/shaders/compat.hlsli). Several cbuffers of a stage
alias it, and some overlap in it on purpose (one shader's c0 is not another's).
SPIR-V allows that, since two blocks at one binding are two views of one buffer, but
DXIL refuses two cbuffers at one register of one space. So the k-th cbuffer declared at
register bN goes to bN in space k: what binds the register file binds it at bN in every
space 0..k. Prints the number of spaces used at b0 and at b1 ("b0 B1"), which hlsl-rs.sh
writes into the _dxil.rs file as CB_SPACES.
"""
import re
import sys

MAX_SPACES = 8


def main():
    src, out = sys.argv[1], sys.argv[2]
    text = open(src).read()
    count = {}

    def place(m):
        reg = int(m.group(2))
        k = count.get(reg, 0)
        count[reg] = k + 1
        return f'{m.group(1)}register(b{reg}, space{k})'

    text = re.sub(r'(\bcbuffer\s+\w+\s*:\s*)register\(\s*b(\d+)\s*\)', place, text)
    for reg, n in count.items():
        if reg > 1:
            sys.exit(f'dxil-spaces.py: a cbuffer at b{reg}: only b0 (vertex) and b1 (pixel) are laid out')
        if n > MAX_SPACES:
            sys.exit(f'dxil-spaces.py: {n} cbuffers at b{reg}, more than the {MAX_SPACES} spaces the root signature has')
    if re.search(r'\bcbuffer\s+\w+\s*\{', text):
        sys.exit('dxil-spaces.py: a cbuffer without a register')
    open(out, 'w').write(text)
    print(count.get(0, 0), count.get(1, 0))


if __name__ == '__main__':
    main()
