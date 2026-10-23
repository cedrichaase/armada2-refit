#!/usr/bin/env bash
# Checks gpu's SPIR-V reflection and patches (src/vulkan/spirv.rs) on the host against
# every shader of render3d and the UI: each pixel shader is linked to every vertex
# shader of its file and given an alpha test, and the result must pass spirv-val.
# Needs rustc and spirv-val (spirv-tools). Writes only under a temporary directory.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/.." && pwd)"
tmp="$(mktemp -d)"
trap '[ -n "${KEEP:-}" ] || rm -rf "$tmp"' EXIT
if [ -n "${KEEP:-}" ]; then echo "kept $tmp"; fi
command -v spirv-val >/dev/null || { echo "spirv-selftest: no spirv-val (Arch: spirv-tools)" >&2; exit 1; }

cat > "$tmp/main.rs" <<EOF
extern crate alloc;
#[path = "$here/src/vulkan/spirv.rs"]
#[allow(dead_code)]
mod spirv;
use std::io::Read;
fn words(p: &str) -> Vec<u32> {
    let mut b = Vec::new();
    std::fs::File::open(p).unwrap().read_to_end(&mut b).unwrap();
    b.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}
fn save(p: &str, w: &[u32]) {
    let b: Vec<u8> = w.iter().flat_map(|x| x.to_le_bytes()).collect();
    std::fs::write(p, b).unwrap();
}
fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a[1].as_str() {
        "reflect" => {
            let r = spirv::reflect(&words(&a[2])).expect("not SPIR-V");
            println!("{:?}", r);
        }
        "link" => {
            let vs = spirv::reflect(&words(&a[2])).unwrap();
            save(&a[4], &spirv::link(&words(&a[3]), &vs.outputs));
        }
        "alpha" => {
            let (lo, hi, inv) = spirv::alpha_interval(6, 128).unwrap();
            match spirv::alpha_test(&words(&a[2]), lo, hi, inv) {
                Some(w) => save(&a[3], &w),
                None => { println!("no alpha test for {}", a[2]); std::process::exit(3) }
            }
        }
        _ => unreachable!(),
    }
}
EOF
rustc -O --edition 2021 -o "$tmp/t" "$tmp/main.rs" 2>&1 | grep -v '^$' || true
[ -x "$tmp/t" ] || { echo "spirv-selftest: did not build" >&2; exit 1; }

python3 - "$repo" "$tmp" <<'EOF'
import re, struct, sys, glob, os
repo, tmp = sys.argv[1], sys.argv[2]
for f in sorted(glob.glob(repo + '/render3d/src/shaders/*_spv.rs') + glob.glob(repo + '/ui/src/shaders/*_spv.rs')):
    base = os.path.basename(f)[:-3]
    for m in re.finditer(r'pub static (\w+)_SPV: \[u32; \d+\] = \[ // (\w+), (\w)s_6_0(.*?)\];', open(f).read(), re.S):
        ws = [int(x, 16) for x in re.findall(r'0x[0-9a-f]+', m.group(4))]
        open('%s/%s.%s.%s.spv' % (tmp, base, m.group(1), m.group(3)), 'wb').write(struct.pack('<%dI' % len(ws), *ws))
EOF

fail=0 n=0
for ps in "$tmp"/*.p.spv; do
    file=$(basename "$ps" | cut -d. -f1)
    "$tmp/t" reflect "$ps" > /dev/null || { echo "reflect failed: $ps"; fail=1; }
    for vs in "$tmp/$file".*.v.spv; do
        [ -e "$vs" ] || continue
        "$tmp/t" link "$vs" "$ps" "$tmp/linked.spv"
        spirv-val --target-env vulkan1.1 "$tmp/linked.spv" || { echo "link invalid: $vs $ps"; fail=1; }
        n=$((n + 1))
    done
    if "$tmp/t" alpha "$ps" "$tmp/alpha.spv" > "$tmp/out"; then
        spirv-val --target-env vulkan1.1 "$tmp/alpha.spv" || { echo "alpha test invalid: $ps"; fail=1; }
    else
        cat "$tmp/out"
    fi
done
for vs in "$tmp"/*.v.spv; do "$tmp/t" reflect "$vs" > /dev/null || { echo "reflect failed: $vs"; fail=1; }; done
echo "spirv-selftest: $n links checked; $( [ $fail = 0 ] && echo ok || echo FAILED )"
exit $fail
