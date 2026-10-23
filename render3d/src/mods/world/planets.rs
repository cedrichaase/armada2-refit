//! Planets: the class planets drawn on a sphere of our own, lit per pixel with
//! `Lighting.asi`'s planet look (a night side, dusk, the limb's haze, the air past the limb, a glint off water,
//! city lights at night) and, where a class has a recipe, `Surfaces.asi`'s ground.
//!
//! The engine draws a planet as a `Planet_Database` of two `GroundMesh` hemispheres and
//! an `Atmosphere` cloud shell, re-tessellated against the camera every frame
//! (`GroundMesh::Recompute`). Its shape is a sphere; what matters of it is the
//! textures' layout, which `GroundMesh::mSetUVs` gives (README, "Planets"), and which
//! our sphere carries in its vertices, so the stock gore textures land where they did.
use super::surfaces::{self, Surfaces};
use crate::engine::{self, field, field_f32, field_u8, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{self, M34, V3, v3};
use crate::shaders::world_planet::*;
use crate::shaders::world_planet_spv::*;
use crate::shaders::world_planet_dxil::*;
use crate::{assets, log, rt, scene};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use gpu::{
    attr, Address, AttrFormat, Blend, Block, BufferKind, Buffer, Cmd, Compare, Cull, Depth, Filter, Format,
    MipFilter, Pipeline, PipelineDesc, Sampler, Semantic, Texture, Topology, VertexAttr,
};

pub const VT_PLANET: usize = 0x6b2b3c;
const VT_PLANET_DB: usize = 0x6bab74;
/// `PlanetInstance`: the instance a planet's fog-of-war spirit carries a copy of.
const VT_PLANET_INSTANCE: usize = 0x6b2ce4;
const FN_DISTANCE_CULL: usize = 0x4d5bb0; // GameObjectInstance::DistanceCull(eye, pos), thiscall
/// cfgPOP_HEAVY: the population a colony's city map is full at (RTS_CFG.h).
const POP_HEAVY: usize = 0x70213c;

const PI: f32 = core::f32::consts::PI;
/// Our sphere: segments of longitude per quarter turn, and of the polar angle pole to
/// pole (a multiple of 4, so rows fall on the gores' corners at 45 and 135 degrees).
const NL: usize = 32;
const NT: usize = 96;

#[repr(C)]
#[derive(Clone, Copy)]
struct Vtx {
    pos: [f32; 3],
    uv: [f32; 2],
    aux: [f32; 3],
}

static LAYOUT: [VertexAttr; 3] = [
    attr(0, AttrFormat::Float3, Semantic::Position, 0),
    attr(12, AttrFormat::Float2, Semantic::TexCoord, 0),
    attr(20, AttrFormat::Float3, Semantic::TexCoord, 1),
];

struct Sphere {
    vb: Buffer,
    ib: Buffer,
    verts: u32,
    tris: u32,
}

/// The pixel shaders, by the index the pipelines' keys carry.
const PS: [gpu::Code; 5] = [gpu::code!(PS_GROUND, PS_GROUND_SPV, PS_GROUND_DXIL), gpu::code!(PS_CITY, PS_CITY_SPV, PS_CITY_DXIL), gpu::code!(PS_CLOUD, PS_CLOUD_SPV, PS_CLOUD_DXIL), gpu::code!(PS_HALO, PS_HALO_SPV, PS_HALO_DXIL), gpu::code!(PS_SURFACE, PS_SURFACE_SPV, PS_SURFACE_DXIL)];
const P_GROUND: usize = 0;
const P_CITY: usize = 1;
const P_CLOUD: usize = 2;
const P_HALO: usize = 3;
const P_SURFACE: usize = 4;
/// How a pass blends: opaque writing depth, alpha-blended or added writing none.
const OPAQUE: u8 = 0;
const ALPHA: u8 = 1;
const ADD: u8 = 2;

/// The planets' samplers 0 and 1: filtered through their levels, addressed as each
/// pass says.
const fn tex_sampler(u: Address, v: Address) -> Sampler {
    gfx::sampler(Filter::Linear, MipFilter::Linear, u, v)
}
const CLAMP: Sampler = tex_sampler(Address::Clamp, Address::Clamp);

/// A unit sphere's share as one GroundMesh holds it: `count` quarter turns of longitude
/// from quarter `start`, the UVs of a texture holding `n` quarters (2 for a hemisphere,
/// 4 for the shell). Per quarter q, at longitude l into it and polar angle t from +y:
/// u = lerp((2q+1)/8, (q pi/2 + l) / 2pi, w) 4/n, v = t/pi, with w rising from 0 at a
/// pole to 1 at 45 degrees from it (the gores' diamonds).
fn sphere(start: usize, count: usize, n: usize) -> Sphere {
    let mut v = Vec::new();
    let mut idx: Vec<u16> = Vec::new();
    for q in 0..count {
        let base = v.len();
        for j in 0..=NL {
            let l = j as f32 / NL as f32 * PI * 0.5;
            let lon = (start + q) as f32 * PI * 0.5 + l;
            let local = q as f32 * PI * 0.5 + l;
            for i in 0..=NT {
                let t = i as f32 / NT as f32 * PI;
                let (st, ct) = (math::sinf(t), math::cosf(t));
                let w = (t / (PI * 0.25)).min((PI - t) / (PI * 0.25)).min(1.0);
                let centre = (2 * q + 1) as f32 / 8.0;
                let u = (centre + (local / (2.0 * PI) - centre) * w) * (4.0 / n as f32);
                let band = if w >= 0.999 { 1.0 } else { 0.0 };
                v.push(Vtx {
                    pos: [math::cosf(lon) * st, ct, math::sinf(lon) * st],
                    uv: [u, t / PI],
                    aux: [t, local, band],
                });
            }
        }
        let row = NT + 1;
        for j in 0..NL {
            for i in 0..NT {
                let a = (base + j * row + i) as u16;
                let b = (base + (j + 1) * row + i) as u16;
                // (A, B, C), (B, D, C): clockwise seen from outside, Direct3D's front.
                idx.extend_from_slice(&[a, b, a + 1, b, b + 1, a + 1]);
            }
        }
    }
    let vb = gfx::buffer(&v, BufferKind::Vertex);
    let ib = gfx::buffer(&idx, BufferKind::Index16);
    Sphere { vb, ib, verts: v.len() as u32, tris: idx.len() as u32 / 3 }
}

/// The look's settings (`Lighting.ini`'s planet keys, under `[World]` in Render3D.ini).
struct Look {
    ambient: [f32; 3],
    diffuse: [f32; 3],
    dusk: [f32; 3],
    wrap: f32,
    fill: f32,
    sun: f32,
    haze: f32,
    haze_col: Option<[f32; 3]>,
    haze_pow: f32,
    glint: f32,
    glint_pow: f32,
    city: f32,
    city_col: [f32; 3],
    halo: f32,
    halo_h: f32,
    halo_fwd: f32,
    /// A fog-of-war ghost's exposure.
    ghost: f32,
}

impl Look {
    fn read() -> Look {
        let ini = crate::config::ini();
        let s = "World";
        let hc = ini.floats(s, "PlanetHazeColour", [-1.0, 0.0, 0.0]);
        Look {
            ambient: ini.floats(s, "PlanetAmbient", [0.018, 0.018, 0.027]),
            diffuse: ini.floats(s, "PlanetDiffuse", [1.0, 1.0, 1.0]),
            dusk: ini.floats(s, "PlanetDusk", [1.0, 0.55, 0.35]),
            wrap: ini.float(s, "PlanetWrap", 0.25),
            fill: ini.float(s, "PlanetFill", 0.315),
            sun: ini.float(s, "PlanetSun", 1.2),
            haze: ini.float(s, "PlanetHaze", 0.45),
            haze_col: if hc[0] >= 0.0 { Some(hc) } else { None },
            haze_pow: ini.float(s, "PlanetHazePower", 3.0),
            glint: ini.float(s, "PlanetGlint", 0.30),
            glint_pow: ini.float(s, "PlanetGlintPower", 40.0),
            city: ini.float(s, "CityLights", 0.8),
            city_col: ini.floats(s, "CityLightColour", [1.0, 0.72, 0.38]),
            halo: ini.float(s, "PlanetHalo", 1.0),
            halo_h: ini.float(s, "PlanetHaloHeight", 0.04),
            halo_fwd: ini.float(s, "PlanetHaloForward", 2.0),
            ghost: ini.float(s, "PlanetGhost", 0.2),
        }
    }
}

/// A planet's own picture of where its people live, per hemisphere: the engine's
/// `cityAllocTexture` (`PlanetInstance::mSetupHemisphere`), made again from the same
/// `CityAllocArray` and population.
struct City {
    tex: Texture,
    pop: f32,
    w: u32,
    h: u32,
    frame: u32,
}

/// What a planet's body is drawn from (`Planets::draw_body`).
struct Body<'a> {
    ground: &'a str,
    atmo: &'a str,
    world: M34,
    shell_world: M34,
    /// The hemispheres' radii (0: none) and the shell's, in the model's units.
    ground_r: f32,
    ground_r2: f32,
    shell_r: f32,
    turb: bool,
    turb_t: f32,
    tint: [f32; 3],
    /// The planet and its race, for the cities.
    people: Option<(usize, usize)>,
    /// What every pass's colour is scaled by: 1, or a ghost's `PlanetGhost`.
    exposure: f32,
}

pub struct Planets {
    spheres: Option<[Sphere; 3]>,
    pipes: Pipes,
    look: Look,
    /// The mean colour of a ground texture's lit texels, by name.
    ground_mean: BTreeMap<String, [f32; 3]>,
    cities: BTreeMap<(usize, usize), City>,
    said: u32,
}

impl Planets {
    pub fn new() -> Planets {
        Planets {
            spheres: None,
            pipes: Pipes::new(),
            look: Look::read(),
            ground_mean: BTreeMap::new(),
            cities: BTreeMap::new(),
            said: 0,
        }
    }

    /// The pipeline of pixel shader `ps` (`P_*`) blending as `mode` says: the sphere,
    /// depth-tested against the scene, its outside's faces (`CULL_CCW`).
    fn pipe(&mut self, ps: usize, mode: u8) -> Option<Pipeline> {
        self.pipes.view((ps * 4 + mode as usize) as u64, || {
            let mut d = PipelineDesc::new(gpu::code!(VS_PLANET, VS_PLANET_SPV, VS_PLANET_DXIL), PS[ps], &LAYOUT, 32, Topology::TriangleList, Format::Unknown);
            d.depth = Depth { test: true, write: mode == OPAQUE, compare: Compare::LessEqual, bias: 0.0, slope_bias: 0.0 };
            d.cull = Cull::CounterClockwise;
            d.blend = match mode {
                OPAQUE => None,
                ALPHA => Some(Blend::ALPHA),
                _ => Some(Blend::ADD),
            };
            d
        })
    }

    fn spheres(&mut self) -> Option<&[Sphere; 3]> {
        if self.spheres.is_none() {
            self.spheres = Some([sphere(0, 2, 2), sphere(2, 2, 2), sphere(0, 4, 4)]);
        }
        self.spheres.as_ref()
    }

    fn note(&mut self, bit: u32, what: &str) {
        if self.said & (1 << bit) == 0 {
            self.said |= 1 << bit;
            log!("world: planets: {}", what);
        }
    }

    /// The mean colour of a texture's lit texels (any channel sum of 24 or more), as
    /// `Lighting.asi`'s glow colour reads it; None if the file does not read.
    fn tex_mean(name: &str) -> Option<[f32; 3]> {
        let mut p = rt::game_path("Textures\\RGB\\");
        p.push_str(name);
        p.push_str(".tga");
        let img = a2fmt::tga::decode(&rt::read_file(&p)?).ok()?;
        let (mut s, mut n) = ([0f64; 3], 0f64);
        for px in img.pixels.chunks_exact(4) {
            let (b, g, r) = (px[0] as u32, px[1] as u32, px[2] as u32);
            if r + g + b < 24 {
                continue;
            }
            s[0] += r as f64;
            s[1] += g as f64;
            s[2] += b as f64;
            n += 1.0;
        }
        if n < 1.0 {
            return None;
        }
        Some([(s[0] / n / 255.0) as f32, (s[1] / n / 255.0) as f32, (s[2] / n / 255.0) as f32])
    }

    /// The ground's colour (`Lighting.asi`'s `glow_colour`): both hemispheres' textures,
    /// else the name alone, else grey.
    fn ground_colour(&mut self, ground: &str, atmo: &str) -> [f32; 3] {
        let name = if !ground.is_empty() { ground } else { atmo };
        if name.is_empty() {
            return [0.5; 3];
        }
        if let Some(c) = self.ground_mean.get(name) {
            return *c;
        }
        let a = Self::tex_mean(&super::name(name, "", Some(1)));
        let b = Self::tex_mean(&super::name(name, "", Some(2)));
        let c = match (a, b) {
            (Some(a), Some(b)) => [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), 0.5 * (a[2] + b[2])],
            (Some(a), None) => a,
            _ => Self::tex_mean(name).unwrap_or([0.5; 3]),
        };
        log!("world: planet {}: ground colour {:.2} {:.2} {:.2}", name, c[0], c[1], c[2]);
        self.ground_mean.insert(String::from(name), c);
        c
    }

    /// A planet's ground colour, for its glow (`lights.rs`).
    pub fn glow_colour(&mut self, o: usize) -> [f32; 3] {
        let cls = obj::class(o);
        if cls == 0 {
            return [0.5; 3];
        }
        let ground = unsafe { rt::cstr_at((cls + 0x4bc) as *const u8, 16) };
        let atmo = unsafe { rt::cstr_at((cls + 0x4ac) as *const u8, 16) };
        self.ground_colour(ground, atmo)
    }

    /// The planet's city picture for hemisphere `h`, made again when its population moves.
    fn city(&mut self, o: usize, race: usize, h: usize, frame: u32) -> Option<Texture> {
        let arr = field(race, 0x514 + h * 4);
        let (w, hh) = (field(arr, 0) as u32, field(arr, 4) as u32);
        let data = field(arr, 8);
        if arr == 0 || data == 0 || w == 0 || hh == 0 || w > 1024 || hh > 1024 {
            return None;
        }
        let pop = field_f32(o, 0x2c8);
        let key = (o, h);
        if let Some(c) = self.cities.get(&key) {
            if c.w == w && c.h == hh && (c.pop == pop || frame.wrapping_sub(c.frame) < 30) {
                return Some(c.tex.clone());
            }
        }
        // mSetupHemisphere: each texel's alpha is 0.4 x clamp((P - value) k), with k the
        // race's +0x470 and P the population over cfgPOP_HEAVY (0x70213c, set from
        // RTS_CFG.h at load: 25000 in the exe's data is not what the game runs with)
        // plus 1 / k.
        let k = field_f32(race, 0x470);
        let heavy = unsafe { rd::<i32>(POP_HEAVY) }.max(1) as f32;
        let p = pop / heavy + if k != 0.0 { 1.0 / k } else { 0.0 };
        let mut px = Vec::with_capacity((w * hh * 4) as usize);
        for y in 0..hh {
            for x in 0..w {
                let v: f32 = unsafe { rd(data + ((w * y + x) * 4) as usize) };
                let c = math::saturate((p - v) * k);
                let a = (0.4 * c * 255.9) as u8;
                px.extend_from_slice(&[255, 255, 255, a]);
            }
        }
        let tex = gfx::texture_bgra8(w, hh, &[&px]);
        self.cities.insert(key, City { tex: tex.clone(), pop, w, h: hh, frame });
        Some(tex)
    }

    /// Forgets planets that are gone (their city pictures), now and then.
    pub fn end_frame(&mut self, frame: u32) {
        if frame % 600 == 0 {
            self.cities.retain(|_, c| frame.wrapping_sub(c.frame) < 600);
        }
    }

    /// Draws a planet in place of `GameObject::Render`; false leaves it to the engine.
    pub fn draw(&mut self, o: usize, v: &View, surf: &mut Surfaces) -> bool {
        let inst = obj::instance(o);
        let db = field(inst, 0x80);
        if db == 0 || engine::vtable(db) != VT_PLANET_DB {
            return false;
        }
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        // PlanetInstance::RenderInternal draws only while the planet's flags (+0x14)
        // have bit 2 and its byte +0x113 is clear.
        if obj::flags(o) & 4 == 0 || field_u8(o, 0x113) != 0 {
            return true;
        }
        let race = field(o, 0x2c4);
        let tint = if race != 0 { Self::race_tint(obj::class(o), race, field_f32(o, 0x2c8)) } else { [1.0; 3] };
        let people = if race != 0 && field_u8(race, 0x452) != 0 { Some((o, race)) } else { None };
        self.draw_instance(v, surf, obj::class(o), inst, db, obj::transform(o), tint, people, 1.0)
    }

    /// A planet's fog-of-war ghost (a spirit of `Ghost::ghostList`: `+4` its class,
    /// `+0x10` a copy of its `PlanetInstance`), drawn on our sphere as the planet itself,
    /// dimmed as stock dims it (`PlanetGhost`: on the bench stock's ghost is a fifth as
    /// bright as the planet seen), without the race's tint or cities, which live on the
    /// object. False if it is not a planet's (the caller leaves it to the engine).
    pub fn draw_ghost(&mut self, sp: usize, v: &View, surf: &mut Surfaces) -> bool {
        let inst = field(sp, 0x10);
        if inst == 0 || engine::vtable(inst) != VT_PLANET_INSTANCE {
            return false;
        }
        let db = field(inst, 0x80);
        let cls = field(sp, 4);
        if db == 0 || cls == 0 || engine::vtable(db) != VT_PLANET_DB {
            return false;
        }
        // GameObjectSpirit::Render's cull (it writes only the spirit's own instance).
        let pos: V3 = unsafe { rd(inst + 0x68) };
        let culled = unsafe {
            let f: unsafe extern "thiscall" fn(usize, *const V3, *const V3) -> u8 = core::mem::transmute(FN_DISTANCE_CULL);
            f(inst, &v.eye, &pos) != 0
        };
        if culled {
            return true;
        }
        let world: M34 = unsafe { rd(inst + 0x44) };
        let exposure = self.look.ghost;
        self.note(7, "a fog-of-war ghost");
        self.draw_instance(v, surf, cls, inst, db, world, [1.0; 3], None, exposure)
    }

    /// The atmosphere's tint (mSetupAtmosphere): white, towards the race's colour as
    /// the population nears the class's maximum.
    fn race_tint(cls: usize, race: usize, pop: f32) -> [f32; 3] {
        let mut tint = [1.0f32; 3];
        let lvl = unsafe { rd::<i32>(cls + 0x490) };
        let maxpop = match lvl {
            1..=4 => (unsafe { rd::<i32>(0x702130 + (lvl as usize - 1) * 4) }) as f32,
            _ => 0.0,
        };
        if maxpop > 0.0 {
            let p = pop / maxpop;
            let rt_: [f32; 3] = unsafe { rd(race + 0x464) };
            for i in 0..3 {
                tint[i] = (1.0 - p) + p * rt_[i];
            }
        }
        tint
    }

    /// A planet from its class, its instance (`PlanetInstance`) and database, at `world`.
    #[allow(clippy::too_many_arguments)]
    fn draw_instance(&mut self, v: &View, surf: &mut Surfaces, cls: usize, inst: usize, db: usize, world: M34, tint: [f32; 3], people: Option<(usize, usize)>, exposure: f32) -> bool {
        if self.spheres().is_none() {
            self.note(0, "the sphere could not be made: the engine draws planets");
            return false;
        }
        let ground = unsafe { rt::cstr_at((cls + 0x4bc) as *const u8, 16) };
        let atmo = unsafe { rt::cstr_at((cls + 0x4ac) as *const u8, 16) };
        let (h0, h1, shell) = (field(db, 0xa8), field(db, 0xac), field(db, 0xb0));
        let rg = if h0 != 0 { field_f32(h0, 0x150) } else { 0.0 };
        let rs = if shell != 0 { field_f32(shell, 0x150) } else { 0.0 };

        // The atmosphere turns about the planet's axis (SetAtmosphereParameters: a yaw of
        // minus the instance's +0xb4 times the database's +0xa4), and its turbulence runs
        // on the instance's own clock (+0xb8).
        let yaw = -field_f32(inst, 0xb4) * field_f32(db, 0xa4);
        let (sy, cy) = (math::sinf(yaw), math::cosf(yaw));
        let yawm = M34 { r: [v3(cy, 0.0, -sy), v3(0.0, 1.0, 0.0), v3(sy, 0.0, cy)], p: V3::ZERO };
        let shell_world = yawm.then(&world);
        let turb_t = field_f32(inst, 0xb8);
        let turb = field_f32(cls, 0x4cc) > 0.0;

        let b = Body {
            ground,
            atmo,
            world,
            shell_world,
            ground_r: if h0 != 0 { rg } else { 0.0 },
            ground_r2: if h1 != 0 { field_f32(h1, 0x150) } else { 0.0 },
            shell_r: if shell != 0 { rs } else { 0.0 },
            turb,
            turb_t,
            tint,
            people,
            exposure,
        };
        self.draw_body(v, surf, &b)
    }

    /// A planet of the menu scene (`crate::menu`), with no game object behind it: the
    /// class's ground (`<ground>1`, `<ground>2`, or its recipe) and cloud shell on a
    /// sphere of `radius` at `world`, the shell turned `yaw` about the axis.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_menu(&mut self, v: &View, surf: &mut Surfaces, ground: &str, atmo: &str, world: &M34, radius: f32, yaw: f32) -> bool {
        if self.spheres().is_none() {
            return false;
        }
        let (sy, cy) = (math::sinf(yaw), math::cosf(yaw));
        let yawm = M34 { r: [v3(cy, 0.0, -sy), v3(0.0, 1.0, 0.0), v3(sy, 0.0, cy)], p: V3::ZERO };
        let b = Body {
            ground,
            atmo,
            world: *world,
            shell_world: yawm.then(world),
            ground_r: if ground.is_empty() { 0.0 } else { radius },
            ground_r2: if ground.is_empty() { 0.0 } else { radius },
            // The shell sits just over the ground, as a class planet's does.
            shell_r: if atmo.is_empty() { 0.0 } else if ground.is_empty() { radius } else { radius * 1.01 },
            turb: false,
            turb_t: 0.0,
            tint: [1.0; 3],
            people: None,
            exposure: 1.0,
        };
        self.draw_body(v, surf, &b)
    }

    /// A planet's body: the ground's two hemispheres (and its cities), the cloud shell,
    /// the air past the limb. False leaves it to the engine.
    fn draw_body(&mut self, v: &View, surf: &mut Surfaces, b: &Body) -> bool {
        let (ground, atmo, world, shell_world) = (b.ground, b.atmo, b.world, b.shell_world);
        let (rg, rs, turb, turb_t, tint) = (b.ground_r, b.shell_r, b.turb, b.turb_t, b.tint);
        let wscale = world.r[0].len();
        let centre = world.p;
        let ground_sf = surf.has(ground);
        let atmo_sf = surf.has(atmo);
        let gc = self.ground_colour(ground, atmo);
        let mut k = self.consts(v, centre, rg.max(rs) * wscale, gc);
        k[14 * 4 + 3] = b.exposure;
        let Some(mut cmd) = gfx::cmd() else { return false };
        let cmd = &mut cmd;
        // The surfaces this body reads, baked first if they are not yet (a bake draws
        // into targets of its own and begins the view's pass again).
        if ground_sf && (rg > 0.0 || b.ground_r2 > 0.0 || rs > 0.0) {
            surf.get(cmd, v, ground);
        }
        if atmo_sf && rs > 0.0 {
            surf.get(cmd, v, atmo);
        }
        if !self.pipe(P_GROUND, OPAQUE).is_some_and(|p| p.ready(gfx::gpu())) {
            return false;
        }
        cmd.vs_block(Block::new(4, &gfx::vp_constants(v)));

        // The ground: two hemispheres, the class's ground texture with 1 and 2 appended.
        for (h, r) in [(0usize, rg), (1usize, b.ground_r2)] {
            if !(r > 0.0) {
                continue;
            }
            let mut sf = None;
            if ground_sf {
                if let Some(s) = surf.get(cmd, v, ground) {
                    surfaces::bind(cmd, s, 2, 3);
                    surf_consts(&mut k, &world, s);
                    sf = self.pipe(P_SURFACE, OPAQUE);
                    self.note(1, "a ground from its recipe");
                }
            }
            let pipe = match sf {
                Some(p) => p,
                None => {
                    let tex = assets::texture(&super::name(ground, "", Some(h + 1)));
                    cmd.bind_texture(0, tex.as_ref(), &CLAMP);
                    let Some(p) = self.pipe(P_GROUND, OPAQUE) else { return false };
                    self.note(2, "a ground from its texture");
                    p
                }
            };
            set_material(&mut k, self.look.ambient, self.look.diffuse);
            k[47 * 4] = 1.0;
            cmd.ps_block(Block::new(0, &k));
            if !cmd.set_pipeline(&pipe) {
                return false;
            }
            self.draw_sphere(cmd, h, &world, r, false, 0.0);
            // The cities, blended over the ground (pass 1 of ST3D_PlanetaryMaterial): the
            // race's development texture times the planet's own picture of its people.
            if let Some((o, race)) = b.people {
                let rname = unsafe { rt::cstr_at((race + 0x452) as *const u8, 32) };
                let dtex = assets::texture(&super::name("PD_", rname, Some(h + 1)));
                let ctex = self.city(o, race, h, v.frame);
                if let (Some(d), Some(c)) = (dtex, ctex) {
                    if let Some(p) = self.pipe(P_CITY, ALPHA).filter(|p| cmd.set_pipeline(p)) {
                        let _ = p;
                        cmd.bind_texture(0, Some(&d), &CLAMP);
                        cmd.bind_texture(1, Some(&c), &CLAMP);
                        self.draw_sphere(cmd, h, &world, r, false, 0.0);
                        self.note(3, "a colony's cities");
                    }
                }
            }
        }

        // The shell: the class's atmosphere texture blended by its alpha, writing no
        // depth; a gas giant's own surface when the atmosphere has a recipe (opaque).
        if rs > 0.0 {
            let mut sf = None;
            if atmo_sf {
                if let Some(s) = surf.get(cmd, v, atmo) {
                    surfaces::bind(cmd, s, 2, 3);
                    surf_consts(&mut k, &shell_world, s);
                    sf = self.pipe(P_SURFACE, OPAQUE);
                    self.note(4, "a gas giant from its recipe");
                }
            }
            // The shell's material: SetAtmosphereTint writes 0.5 and 0.75 x the tint; the
            // ambient is PlanetAmbient x tint.
            let amb = [self.look.ambient[0] * tint[0], self.look.ambient[1] * tint[1], self.look.ambient[2] * tint[2]];
            set_material(&mut k, amb, tint);
            let sf_ok = sf.is_some();
            let pipe = match sf {
                Some(p) => {
                    k[47 * 4] = 1.0;
                    p
                }
                None => {
                    let tex = assets::texture(atmo);
                    cmd.bind_texture(0, tex.as_ref(), &tex_sampler(Address::Wrap, Address::Clamp));
                    let Some(p) = self.pipe(P_CLOUD, ALPHA) else { return true };
                    // Over a ground from a recipe, which brings clouds of its own, only the
                    // shell's halo at the limb is left.
                    k[47 * 4] = if ground_sf && surf.get(cmd, v, ground).is_some() { 0.0 } else { 1.0 };
                    self.note(5, "a cloud shell");
                    p
                }
            };
            if !cmd.set_pipeline(&pipe) {
                return true;
            }
            cmd.ps_block(Block::new(0, &k));
            self.draw_sphere(cmd, 2, &shell_world, rs, turb && !sf_ok, turb_t);

            // The air past the limb: the shell's sphere pushed out to PlanetHaloHeight
            // over the ground, added, no depth written.
            if self.look.halo > 0.0 && rg > 1e-3 && rs > 1e-3 {
                if let Some(p) = self.pipe(P_HALO, ADD).filter(|p| cmd.set_pipeline(p)) {
                    let _ = p;
                    let g = rg * wscale;
                    let top = g * (1.0 + self.look.halo_h.max(1e-3));
                    k[48 * 4..48 * 4 + 4].copy_from_slice(&[g, top, self.look.halo, self.look.halo_fwd]);
                    cmd.ps_block(Block::new(0, &k));
                    let r = (top * 1.03 / wscale).max(rs);
                    self.draw_sphere(cmd, 2, &shell_world, r, false, 0.0);
                    self.note(6, "the air past a limb");
                }
            }
        }
        true
    }

    fn draw_sphere(&self, cmd: &mut Cmd, which: usize, world: &M34, radius: f32, turb: bool, t: f32) {
        let Some(s) = self.spheres.as_ref() else { return };
        let s = &s[which];
        cmd.vs_block(Block::new(0, world.to_m4().transpose().as_floats()));
        cmd.vs_block(Block::new(8, &[radius, if turb { 1.0 } else { 0.0 }, t, 0.0]));
        let (Some(vb), Some(ib)) = (cmd.span_of(&s.vb, 32), cmd.span_of(&s.ib, 2)) else { return };
        cmd.draw_indexed(&vb, 0, s.verts, &ib, 0, s.tris * 3);
    }

    /// planet.hlsl's c0..c48 for one planet: lights, the camera, the look.
    fn consts(&self, v: &View, centre: V3, radius: f32, ground: [f32; 3]) -> Vec<f32> {
        let mut k = alloc::vec![0.0f32; 49 * 4];
        let ls = scene::lights();
        // c2..c9: directional lights, the Key (the brightest) first; the others, which on
        // a sphere light a whole hemisphere, times PlanetFill.
        let mut dirs: Vec<_> = ls.dirs.iter().take(4).copied().collect();
        if let Some(key) = ls.key() {
            if let Some(i) = dirs.iter().position(|d| d.colour == key.colour && d.dir == key.dir) {
                dirs.swap(0, i);
            }
        }
        for (i, d) in dirs.iter().enumerate() {
            let m = if i == 0 { self.look.sun } else { self.look.fill };
            k[(2 + i) * 4..(2 + i) * 4 + 3].copy_from_slice(&[d.colour.x * m, d.colour.y * m, d.colour.z * m]);
            let t = -d.dir;
            k[(6 + i) * 4..(6 + i) * 4 + 3].copy_from_slice(&[t.x, t.y, t.z]);
        }
        k[2 * 4 + 3] = self.look.sun;
        k[40..43].copy_from_slice(&[centre.x, centre.y, centre.z]);
        k[44..47].copy_from_slice(&[v.eye.x, v.eye.y, v.eye.z]);
        k[48..52].copy_from_slice(&[self.look.dusk[0], self.look.dusk[1], self.look.dusk[2], self.look.wrap]);
        // c13: the haze: PlanetHazeColour, or half the ground's own hue, half a sky blue.
        let g = self.look.haze_col.unwrap_or_else(|| {
            let sky = [0.40, 0.62, 1.00];
            let peak = ground[0].max(ground[1]).max(ground[2]);
            let mut g = [0.0; 3];
            for i in 0..3 {
                g[i] = 0.5 * if peak > 1e-3 { ground[i] / peak } else { 1.0 } + 0.5 * sky[i];
            }
            g
        });
        k[52..56].copy_from_slice(&[g[0] * self.look.haze, g[1] * self.look.haze, g[2] * self.look.haze, self.look.haze_pow]);
        k[56..60].copy_from_slice(&[self.look.glint, self.look.glint_pow, self.look.city, 1.0]);
        k[60..63].copy_from_slice(&self.look.city_col);
        // c16..c39: the point lights that reach the surface.
        for i in 0..8 {
            k[(32 + i) * 4] = 1.0;
            k[(32 + i) * 4 + 1] = 1.0;
        }
        // Without the planets' own glows, which the CPU path never lit a planet with
        // either.
        let near: Vec<_> = ls.points_near(centre, radius, 16).into_iter().filter(|p| !scene::lights::is_day_side(p.light)).take(8).collect();
        for (i, p) in near.iter().enumerate() {
            k[(16 + i) * 4..(16 + i) * 4 + 3].copy_from_slice(&[p.colour.x, p.colour.y, p.colour.z]);
            k[(24 + i) * 4..(24 + i) * 4 + 3].copy_from_slice(&[p.pos.x, p.pos.y, p.pos.z]);
            k[(32 + i) * 4] = p.start;
            k[(32 + i) * 4 + 1] = if p.range > 1e-3 { 1.0 / p.range } else { 1000.0 };
        }
        k
    }
}

fn set_material(k: &mut [f32], base: [f32; 3], diffuse: [f32; 3]) {
    k[0..3].copy_from_slice(&base);
    k[4..7].copy_from_slice(&diffuse);
}

/// c40..c46: the world matrix's axes, and what the recipe gives the draw.
fn surf_consts(k: &mut [f32], w: &M34, s: &surfaces::Surface) {
    for i in 0..3 {
        k[(40 + i) * 4..(40 + i) * 4 + 4].copy_from_slice(&[w.r[i].x, w.r[i].y, w.r[i].z, 0.0]);
    }
    for i in 0..4 {
        k[(43 + i) * 4..(43 + i) * 4 + 4].copy_from_slice(&s.draw[i]);
    }
}
