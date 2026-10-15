//! The shots' light, at the start of each view (after `Ordnance::PreRenderAll` has
//! registered every torpedo's and pulse's own light):
//!
//! - **Recolouring** (from `Lighting.asi`, lighting/README.md "Torpedoes and pulses"):
//!   an ODF's `lightColor` ignores its projectile (every Federation photon is cyan,
//!   nearly every other torpedo and pulse green). The class's light (class +0x10, an
//!   `ST3D_Point_Light`, its colour +0xf4) takes the hue of its sprite instead: the mean
//!   of the sprite's first frame (its rectangle +0x38 of its texture +0x58) of the
//!   installed TGA, scaled to the ODF colour's peak. Written into this view's registry
//!   record (Storm3D +0x60: record +0 the light, +4 its colour), not into the class.
//! - **Phasers** (also `Lighting.asi`'s): a light at the emitter and one at the impact
//!   of every visible phaser (vtable 0x6b8ee4), in the beam's own colour (its texture's
//!   mean, scaled to a peak of 1, times its tint +0xf0), up in 0.06 s and down over the
//!   last 0.25 s of its life (time left +0xac of the class's lifeSpan +0x1c).
//! - **The special weapons and the warp flash** (Specials.asi's): `fx.rs` says where.
//!
//! The new lights are ordinary `ST3D_Point_Light`s (made with the engine's own
//! constructor, kept for the session) registered with `RegisterLight`, so a hull the
//! engine draws takes them as it takes a torpedo's; then the scene's list is read
//! again for ours. With `Lighting.asi` loaded, it recolours and lights phasers itself
//! and this module leaves both to it.
use super::eng;
use crate::assets;
use crate::engine::{self, field, field_f32, rd};
use crate::math::{v3, M34, V3};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

const FN_NEW: usize = 0x652710; // operator new, cdecl
const FN_POINT_LIGHT: usize = 0x62f240; // ST3D_Point_Light::ST3D_Point_Light(db, node, name), thiscall
const FN_REGISTER_LIGHT: usize = 0x62d250; // ST3D_GraphicsEngine::RegisterLight(light, colour&, Matrix34&), thiscall
const VT_POINT: usize = 0x6bc8ac;
const VT_PHASER: usize = 0x6b8ee4;

pub struct Lights {
    pool: Vec<usize>,
    /// Per texture (lower-case stem): the mean colour of a rectangle (u, v, w, h in
    /// thousandths), or None if it could not be read.
    means: BTreeMap<(String, [i32; 4]), Option<V3>>,
    pub recolour: bool,
    pub phasers: bool,
    /// Phaser lights: emitter brightness, start, range, lift; impact brightness,
    /// start, range, lift (Lighting.ini's defaults).
    pub ph: [f32; 8],
    said: u32,
}

impl Lights {
    pub fn new() -> Lights {
        Lights {
            pool: Vec::new(),
            means: BTreeMap::new(),
            recolour: true,
            phasers: true,
            ph: [4.5, 0.0, 24.0, 3.0, 2.0, 6.0, 70.0, 8.0],
            said: 0,
        }
    }

    fn light(&mut self, n: usize) -> usize {
        while self.pool.len() <= n {
            let l = unsafe {
                let new: unsafe extern "C" fn(u32) -> usize = core::mem::transmute(FN_NEW);
                let l = new(0x138);
                if l == 0 {
                    return 0;
                }
                let ctor: unsafe extern "thiscall" fn(usize, usize, usize, *const u8) = core::mem::transmute(FN_POINT_LIGHT);
                ctor(l, 0, 0, b"ordnance\0".as_ptr());
                l
            };
            self.pool.push(l);
        }
        self.pool[n]
    }

    /// Registers a point light with the engine for this view: full to `start`, gone
    /// `range` further.
    fn register(&mut self, n: usize, pos: V3, col: V3, start: f32, range: f32) {
        let l = self.light(n);
        if l == 0 {
            return;
        }
        unsafe {
            core::ptr::write_unaligned((l + 0x100) as *mut f32, start);
            core::ptr::write_unaligned((l + 0x104) as *mut f32, range.max(0.01));
            let m = M34 { p: pos, ..M34::IDENTITY };
            let c = [col.x, col.y, col.z, 1.0f32];
            let f: unsafe extern "thiscall" fn(usize, usize, *const f32, *const M34) = core::mem::transmute(FN_REGISTER_LIGHT);
            f(engine::storm3d(), l, c.as_ptr(), &m);
        }
    }

    /// The mean colour (0..1) of a rectangle of a texture, from the TGA.
    fn mean(&mut self, tex: &str, r: [f32; 4]) -> Option<V3> {
        let key = (crate::rt::lower(tex), [(r[0] * 1000.0) as i32, (r[1] * 1000.0) as i32, (r[2] * 1000.0) as i32, (r[3] * 1000.0) as i32]);
        if let Some(m) = self.means.get(&key) {
            return *m;
        }
        let m = assets::image(tex).and_then(|img| {
            let (w, h) = (img.width as f32, img.height as f32);
            let x0 = ((r[0] * w + 0.5) as i32).max(0) as u32;
            let y0 = ((r[1] * h + 0.5) as i32).max(0) as u32;
            let x1 = (((r[0] + r[2]) * w + 0.5) as u32).min(img.width);
            let y1 = (((r[1] + r[3]) * h + 0.5) as u32).min(img.height);
            if x1 <= x0 || y1 <= y0 {
                return None;
            }
            let mut acc = [0f64; 3];
            for y in y0..y1 {
                for x in x0..x1 {
                    let p = img.pixel(x, y);
                    acc[0] += p[2] as f64;
                    acc[1] += p[1] as f64;
                    acc[2] += p[0] as f64;
                }
            }
            let n = ((x1 - x0) * (y1 - y0)) as f64 * 255.0;
            Some(v3((acc[0] / n) as f32, (acc[1] / n) as f32, (acc[2] / n) as f32))
        });
        crate::log!("ordnance: light colour of {} {:?}: {:?}", tex, r, m);
        self.means.insert(key, m);
        m
    }

    /// A sprite's texture (its stem) and its first frame's rectangle.
    fn sprite_rect(sprite: usize) -> Option<(String, [f32; 4])> {
        let tex = field(sprite, 0x58);
        let name = unsafe { crate::rt::cstr_at(field(tex, 8) as *const u8, 260) };
        if name.is_empty() {
            return None;
        }
        let base = name.rsplit(['\\', '/']).next().unwrap_or(name);
        let stem = base.split('.').next().unwrap_or(base);
        Some((String::from(stem), unsafe { rd(sprite + 0x38) }))
    }

    /// This view's torpedo and pulse lights in their sprites' colours.
    fn recolour_registry(&mut self) {
        // The ordnance classes' lights in play, with their sprites.
        let mut ls: Vec<(usize, usize)> = Vec::new();
        eng::for_each_ordnance(|o| {
            let cls = field(o, 0x34);
            let l = field(cls, 0x10);
            if l != 0 && engine::vtable(l) == VT_POINT && !ls.iter().any(|e| e.0 == l) {
                ls.push((l, field(cls, 0x12c)));
            }
        });
        if ls.is_empty() {
            return;
        }
        let head = field(engine::storm3d(), 0x60);
        if head == 0 {
            return;
        }
        let mut node = field(head, 0);
        let mut guard = 0;
        while node != head && node != 0 && guard < 4096 {
            guard += 1;
            let rec = field(node, 8);
            node = field(node, 0);
            let l = field(rec, 0);
            let Some(&(_, sprite)) = ls.iter().find(|e| e.0 == l) else { continue };
            let odf: V3 = unsafe { rd(l + 0xf4) };
            let peak = odf.x.max(odf.y).max(odf.z);
            let Some((tex, r)) = Self::sprite_rect(sprite) else { continue };
            let Some(m) = self.mean(&tex, r) else { continue };
            let mp = m.x.max(m.y).max(m.z);
            if peak <= 0.0 || mp <= 0.0 {
                continue;
            }
            let c = m * (peak / mp);
            if self.said < 16 {
                self.said += 1;
                crate::log!("ordnance: light of {} recoloured {:.2} {:.2} {:.2} -> {:.2} {:.2} {:.2}", tex, odf.x, odf.y, odf.z, c.x, c.y, c.z);
            }
            unsafe { core::ptr::write_unaligned((rec + 4) as *mut V3, c) };
        }
    }

    /// The phasers' emitter and impact lights; returns how many lights were used.
    fn phaser_lights(&mut self, mut n: usize) -> usize {
        let mut shots: Vec<(V3, V3, V3, usize)> = Vec::new();
        eng::for_each_ordnance(|o| {
            if engine::vtable(o) != VT_PHASER || eng::byte(o, 0x27) != 0 || eng::byte(o, 0x24) == 0 {
                return;
            }
            let cls = field(o, 0x34);
            let (left, len) = (field_f32(o, 0xac), field_f32(cls, 0x1c));
            if cls == 0 || left <= 0.0 || len <= 0.0 {
                return;
            }
            let age = len - left;
            let mut k = if age < 0.06 { age / 0.06 } else { 1.0 };
            if left < 0.25 {
                k *= left / 0.25;
            }
            if k <= 0.0 {
                return;
            }
            let a: V3 = unsafe { rd(o + 0xbc) };
            let b: V3 = unsafe { rd(o + 0xc8) };
            let tint: V3 = unsafe { rd(o + 0xf0) };
            let t = v3(tint.x.clamp(0.0, 1.0), tint.y.clamp(0.0, 1.0), tint.z.clamp(0.0, 1.0)) * k;
            shots.push((a, b, t, field(cls, 0x12c)));
        });
        for (a, b, t, spr) in shots {
            if n >= 96 {
                break;
            }
            let col = Self::sprite_rect(spr)
                .and_then(|(tex, _)| self.mean(&tex, [0.0, 0.0, 1.0, 1.0]))
                .map(|m| {
                    let p = m.x.max(m.y).max(m.z);
                    if p > 1e-3 { m * (1.0 / p) } else { v3(1.0, 1.0, 1.0) }
                })
                .unwrap_or(v3(1.0, 1.0, 1.0));
            let c = col.mul_comp(t);
            let d = b - a;
            let len = d.len();
            if len < 1e-3 {
                continue;
            }
            let ph = self.ph;
            if ph[0] > 0.0 {
                let p = a + d * (len.min(ph[3]) / len);
                self.register(n, p, c * ph[0], ph[1], ph[2] - ph[1]);
                n += 1;
            }
            if ph[4] > 0.0 {
                let p = b - d * (len.min(ph[7]) / len);
                self.register(n, p, c * ph[4], ph[5], ph[6] - ph[5]);
                n += 1;
            }
        }
        n
    }

    /// The start of a view: recolour, then register the new lights. True if the
    /// registry changed (the scene's list must be read again).
    pub fn begin_view(&mut self, fx: &[super::fx::Lit]) -> bool {
        let lighting = crate::plugin_loaded("Lighting.asi");
        let mut changed = false;
        if self.recolour && !lighting {
            self.recolour_registry();
            changed = true;
        }
        let mut n = 0;
        if self.phasers && !lighting {
            n = self.phaser_lights(n);
        }
        for l in fx {
            if n >= 128 {
                break;
            }
            self.register(n, l.pos, l.col, l.start, l.range);
            n += 1;
        }
        changed || n > 0
    }
}
