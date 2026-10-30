#!/usr/bin/env python3
"""Give a pixel shader an alpha-tested twin, for DXIL (hlsl-rs.sh, an entry marked `:at`).

    dxil-alpha.py <in.hlsl> <out.hlsl> <entry> [<entry> ...]

Direct3D 9 has the alpha test as fixed-function state, and Direct3D 12 does not have it.
The Vulkan backend puts it into the pixel shader's SPIR-V (gpu/src/vulkan/spirv.rs);
DXIL cannot be patched like that (it is LLVM bitcode, and signed), so each pixel shader
a pipeline draws with an alpha test is compiled twice, the second time as
`<entry>__alpha_test`: the entry called as it is, then the fragment discarded as
Direct3D 9's test would, by the interval gpu's Direct3D 12 backend passes in root
constants at b2 (`lo <= a * 255 < hi` keeps it, or the outside of it when `invert`:
`spirv::alpha_interval`). Only `float4 <entry>(...) : SV_Target` is taken.
"""
import re
import sys

CBUFFER = '''
// The alpha test of gpu's Direct3D 12 backend (dxil-alpha.py): lo, hi, invert.
cbuffer gpu_alpha_test : register(b2)
{
    float4 gpu_alpha_test_k;
};
'''

WRAPPER = '''
float4 {entry}__alpha_test({params}) : SV_Target
{{
    float4 gpu_c = {entry}({args});
    float gpu_a = gpu_c.a * 255.0;
    bool gpu_in = gpu_a >= gpu_alpha_test_k.x && gpu_a < gpu_alpha_test_k.y;
    if (gpu_in == (gpu_alpha_test_k.z != 0.0))
        discard;
    return gpu_c;
}}
'''


def main():
    src, out, entries = sys.argv[1], sys.argv[2], sys.argv[3:]
    text = open(src).read()
    text += CBUFFER
    for entry in entries:
        m = re.search(r'\bfloat4\s+' + re.escape(entry) + r'\s*\(([^)]*)\)\s*:\s*SV_Target0?\b', text)
        if not m:
            sys.exit(f'dxil-alpha.py: no "float4 {entry}(...) : SV_Target" to give an alpha test')
        params = m.group(1).strip()
        args = []
        for p in filter(None, (x.strip() for x in params.split(','))):
            p = p.split(':')[0].strip()
            name = re.findall(r'\w+', p)
            if len(name) < 2:
                sys.exit(f'dxil-alpha.py: {entry}: a parameter without a name: {p!r}')
            args.append(name[-1])
        text += WRAPPER.format(entry=entry, params=params, args=', '.join(args))
    open(out, 'w').write(text)


if __name__ == '__main__':
    main()
