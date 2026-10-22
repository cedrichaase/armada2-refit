//! Rocks: the objects of class `Scrap` (the dilithium moons, a lone asteroid, comets,
//! scrap) and the asteroid fields' rocks, drawn from their SODs through the scene layer.
//! A mesh whose texture has a surface recipe (`Mdmoon`, `Master1`, `Master2`) takes its
//! colour, relief and cut-out from the recipe's cubes (`shaders/world_rock.hlsl`, the
//! port of `Lighting.asi`'s `moon_ps`); any other mesh draws as the scene draws a SOD.
//!
//! - `Scrap::Render` sets the colour of the material `glowdmoon` (in the engine's
//!   material database, `Storm3D+0x44`, its diffuse at `+0x24`) to (0, 0.5, 1) times the
//!   share of the moon's dilithium left (`ResourceInterface::GetTotalValue` over the
//!   class's `GetTotalMaxValue`, the object's resource at `+0x17c`), then draws as any
//!   object. The material lights by `LightVertices_Constant`: the glow shell is that one
//!   colour, drawn here added, after the opaque scene.
//! - An asteroid field's instance (`AsteroidFieldInstance`) holds its rocks as an array
//!   of `ST3D_Instance`s (`+0x98`, count `+0x9c`, 0x98 bytes each), each with its own
//!   transform (`+0x44`) and database (`+0x80`). The databases are the class's eight
//!   (`AsteroidFieldClass+0x1e0`), loaded from the names in the table at 0x6b07e8
//!   (`master.sod`, `master1` .. `master5`, `master4` again, `master7`) at the field's
//!   `ScaleSOD`; `AsteroidFieldInstance::RenderInternal` draws each rock its distance
//!   cull keeps.
use super::surfaces::{self, Surfaces};
use gpu::d3d9::raw::*;
use crate::engine::{self, field, field_f32, field_m34, field_u8, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Shader};
use crate::math::{M34, V3, v3};
use crate::scene::{self, pose, Model};
use crate::shaders::world_rock::*;
use crate::{assets, log, rt};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

pub const VT_SCRAP: usize = 0x6b15b4;
pub const VT_ASTEROID_FIELD: usize = 0x6b0cb4;
const ASTEROID_NAMES: usize = 0x6b07e8;
const CFG_CULL_DISTANCE: usize = 0x6fcac0; // cfgOBJECT_CULLING_DISTANCE
const FN_RES_VALUE: usize = 0x4d6820; // ResourceInterface::GetTotalValue, thiscall -> float
const FN_RES_MAX: usize = 0x4cf1f0; // ResourceInterfaceClass::GetTotalMaxValue, thiscall -> float

/// What a part of a model is to us.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// No recipe: the scene's own draw.
    Plain,
    /// Its texture has a recipe.
    Surface,
    /// The dilithium moons' glow shell (material `glowdmoon`).
    Glow,
}

struct Info {
    kinds: Vec<Kind>,
    /// Per part: 1 / its extent about its centre on each object axis.
    ext: Vec<V3>,
    any: bool,
    /// A part has a surface from a recipe.
    surface: bool,
}

/// A sprite node's quad waiting for the translucent step.
struct Spr {
    tex: String,
    /// Added (`additive`, `add_nomipf`, none), else blended by its alpha.
    add: bool,
    corners: [V3; 4],
    uv: [f32; 4],
    colour: [f32; 4],
    depth: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SVert {
    pos: [f32; 3],
    col: [f32; 4],
    uv: [f32; 2],
}

static SPRITE_DECL: [VertexElement; 4] = [
    ve(0, 0, DECLTYPE_FLOAT3, DECLUSAGE_POSITION, 0),
    ve(0, 12, DECLTYPE_FLOAT4, DECLUSAGE_COLOR, 0),
    ve(0, 28, DECLTYPE_FLOAT2, DECLUSAGE_TEXCOORD, 0),
    VE_END,
];

/// A glow shell waiting for the translucent step.
struct Glow {
    model: Rc<Model>,
    part: usize,
    world: M34,
    colour: V3,
}

pub struct Rocks {
    vs: Shader,
    ps: Shader,
    flat: Shader,
    decl: Decl,
    info: BTreeMap<String, Info>,
    scales: BTreeMap<String, f32>,
    glows: Vec<Glow>,
    sprites: Vec<Spr>,
    svs: Shader,
    sps: Shader,
    sdecl: Decl,
    svb: gfx::DynVb,
    look: Look,
    said: u32,
}

/// `Lighting.asi`'s hull look, as its hull shaders lit the moons and asteroids.
struct Look {
    ambient: [f32; 3],
    spec: f32,
    spec_pow: f32,
    rim: [f32; 3],
    rim_pow: f32,
    sun: f32,
    /// The moons' inner glow shell dark and opaque, as under Lighting.asi (`MoonShell=dark`,
    /// the settled look: black cracks); `glow`: stock's, added in the moon's colour.
    dark_shell: bool,
    /// The light in a dilithium moon's cracks, times its nimbus's colour at the peak of
    /// its pulse (`MoonCracks`; 0: the cracks are holes onto the shell, as before).
    cracks: f32,
    /// How much of the nimbus's pulse the cracks take (`MoonPulse`, 0..1).
    pulse: f32,
    /// The white at a crack's core, times the light's brightness (`MoonCrackCore`).
    crack_core: f32,
    /// How deep the cracks are, in the mesh's radii: the light lies at their bottom and
    /// the walls hide it at an angle (`MoonCrackDepth`; 0: flat, the light at the surface).
    crack_depth: f32,
    /// How steeply the light fades up a crack's walls (`MoonCrackFalloff`, an exponent).
    crack_falloff: f32,
    /// The nimbus drawn (`MoonNimbus`): the round halo behind the moon. Off by default,
    /// since the light shows through the cracks.
    nimbus: bool,
}

impl Look {
    fn read() -> Look {
        let ini = crate::config::ini();
        let s = "World";
        Look {
            ambient: ini.floats(s, "RockAmbient", [0.05, 0.05, 0.07]),
            spec: ini.float(s, "RockSpecular", 0.7),
            spec_pow: ini.float(s, "RockSpecularPower", 24.0),
            rim: ini.floats(s, "RockRimLight", [0.12, 0.14, 0.20]),
            rim_pow: ini.float(s, "RockRimPower", 3.0),
            sun: ini.float(s, "RockSun", 1.1),
            dark_shell: !ini.string(s, "MoonShell").is_some_and(|m| m.trim().eq_ignore_ascii_case("glow")),
            cracks: ini.float(s, "MoonCracks", 4.0).max(0.0),
            pulse: ini.float(s, "MoonPulse", 0.5).clamp(0.0, 1.0),
            crack_core: ini.float(s, "MoonCrackCore", 1.0).max(0.0),
            crack_depth: ini.float(s, "MoonCrackDepth", 0.02).max(0.0),
            crack_falloff: ini.float(s, "MoonCrackFalloff", 3.0).max(0.0),
            nimbus: ini.float(s, "MoonNimbus", 0.0) != 0.0,
        }
    }
}

impl Rocks {
    pub fn new() -> Rocks {
        Rocks {
            vs: Shader::vertex(&VS_ROCK),
            ps: Shader::pixel(&PS_ROCK),
            flat: Shader::pixel(&PS_FLAT),
            decl: Decl::new(&scene::VERTEX),
            info: BTreeMap::new(),
            scales: BTreeMap::new(),
            glows: Vec::new(),
            sprites: Vec::new(),
            svs: Shader::vertex(&VS_RSPRITE),
            sps: Shader::pixel(&PS_RSPRITE),
            sdecl: Decl::new(&SPRITE_DECL),
            svb: gfx::DynVb::new(16 * 1024),
            look: Look::read(),
            said: 0,
        }
    }

    /// The look read again (`[World] Reload=1`).
    pub fn reload(&mut self) {
        self.look = Look::read();
    }

    pub fn reset(&mut self) {
        self.glows.clear();
        self.sprites.clear();
        self.svb.reset();
    }

    fn note(&mut self, bit: u32, what: &str) {
        if self.said & (1 << bit) == 0 {
            self.said |= 1 << bit;
            log!("world: rocks: {}", what);
        }
    }

    fn scale_sod(&mut self, odf: &str) -> f32 {
        if let Some(s) = self.scales.get(odf) {
            return *s;
        }
        let s = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD").or_else(|| o.get_f32("scaleSOD"))).filter(|s| *s > 0.0).unwrap_or(1.0);
        self.scales.insert(String::from(odf), s);
        s
    }

    fn info(&mut self, m: &Model, surf: &mut Surfaces) -> &Info {
        if !self.info.contains_key(&m.name) {
            let mut kinds = Vec::new();
            let mut ext = Vec::new();
            for part in &m.parts {
                let tex = part.texture.as_deref().unwrap_or("");
                let k = if !tex.is_empty() && surf.has(tex) {
                    Kind::Surface
                } else if part.groups.iter().any(|g| g.material == "glowdmoon") {
                    Kind::Glow
                } else {
                    Kind::Plain
                };
                kinds.push(k);
                // The extent about the bounding-sphere centre along each object axis.
                let mut e = V3::ZERO;
                if let Some(mesh) = m.sod.nodes.get(part.node).and_then(|n| n.mesh()) {
                    for p in &mesh.positions {
                        e.x = e.x.max((p[0] - part.centre.x).abs());
                        e.y = e.y.max((p[1] - part.centre.y).abs());
                        e.z = e.z.max((p[2] - part.centre.z).abs());
                    }
                }
                ext.push(v3(1.0 / e.x.max(1e-4), 1.0 / e.y.max(1e-4), 1.0 / e.z.max(1e-4)));
            }
            let any = kinds.iter().any(|k| *k != Kind::Plain);
            log!("world: rocks: {}: {} parts, {} (SOD {:.2})", m.name, kinds.len(), if any { "surfaces from recipes" } else { "as the scene draws it" }, m.sod.version);
            for (i, part) in m.parts.iter().enumerate() {
                let n = &m.sod.nodes[part.node];
                log!("world: rocks:   part {} {}: cull {}, blend {:?}, two-sided {}, node det {:.2}, centre {:.2} {:.2} {:.2} radius {:.2}", i, n.name, n.mesh().map_or(9, |x| x.cull), part.blend, part.two_sided, crate::scene::pose::m34(&n.transform).det3(), part.centre.x, part.centre.y, part.centre.z, part.radius);
            }
            let surface = kinds.iter().any(|k| *k == Kind::Surface);
            self.info.insert(m.name.clone(), Info { kinds, ext, any, surface });
        }
        &self.info[&m.name]
    }

    /// A `Scrap` in place of `Scrap::Render`; false leaves it to the engine.
    pub fn draw_scrap(&mut self, o: usize, v: &View, surf: &mut Surfaces) -> bool {
        let name = obj::odf_name(o);
        if name.is_empty() {
            return false;
        }
        let Some(model) = scene::model(v.dev, &crate::assets::sod_of(name)) else { return false };
        // Only a scrap with a surface from a recipe (the dilithium moons): any other is
        // the hulls module's (or the engine's), drawn as any plain SOD.
        if !self.info(&model, surf).surface {
            return false;
        }
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        let inst = obj::instance(o);
        let mut s = self.scale_sod(name);
        let mut world: M34 = obj::transform(o);
        // The scale an Entity sets on its instance (a moon shrinks as it is mined),
        // applied while the instance's flag +0x74 is on.
        let sc = if inst != 0 && field_u8(inst, 0x74) != 0 { unsafe { rd::<V3>(inst + 0x28) } } else { v3(1.0, 1.0, 1.0) };
        if s <= 0.0 {
            s = 1.0;
        }
        world.r[0] = world.r[0] * (sc.x * s);
        world.r[1] = world.r[1] * (sc.y * s);
        world.r[2] = world.r[2] * (sc.z * s);
        let glow = glow_colour(o);
        let clock = pose::Clock::of_instance(inst, v.time);
        let p = pose::pose(&model.sod, &model.info, &world, clock, pose::auto(v.eye));
        // Its sprite nodes (the moon's `oreglow` nimbus), on the instance's clock as
        // GameObject::RenderInternal plays them; their colour is the light in the cracks.
        let (since, flags) = if inst != 0 { ((v.time - unsafe { rd::<f32>(inst + 0x18) }).max(0.0), field(inst, 0xc) as u32) } else { (v.time, 3) };
        let nimbus = self.sprite_nodes(v, &model, &p, sc.x * s, since, flags);
        let core = self.crack_light(glow, nimbus);
        self.draw_model(v, &model, &p, glow, core, surf);
        true
    }

    /// The light inside a dilithium moon's cracks, or None to leave them holes: the
    /// nimbus's colour at the peak of its pulse (stock's (0, 0.5, 1); purple on the
    /// infinite moons), swung by `MoonPulse` of the pulse, times the share of dilithium
    /// left (the glow shell's colour over its full colour) and `MoonCracks`.
    fn crack_light(&self, glow: Option<V3>, nimbus: Option<([f32; 3], [f32; 3])>) -> Option<V3> {
        if self.look.cracks <= 0.0 {
            return None;
        }
        let full = v3(0.0, 0.5, 1.0);
        let left = glow.map_or(1.0, |g| (g.len() / full.len()).min(1.0));
        let (now, peak) = nimbus.unwrap_or(([0.0, 0.5, 1.0], [0.0, 0.5, 1.0]));
        let lum = |c: [f32; 3]| 0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2];
        let swing = if lum(peak) > 1e-4 { (lum(now) / lum(peak)).clamp(0.0, 1.0) } else { 1.0 };
        let k = self.look.cracks * left * (1.0 - self.look.pulse + self.look.pulse * swing);
        Some(v3(peak[0], peak[1], peak[2]) * k)
    }

    /// The model's shown sprite nodes, queued for the translucent step when `MoonNimbus`
    /// is on (as the hulls module draws a hull's: `@sprite_node` id, its sprite's
    /// rectangle, its animation's colour; a billboard's size is a half extent, an
    /// oriented quad's the full one in its node's x-y plane). Gives the first animated
    /// node's colour now and at its brightest key.
    fn sprite_nodes(&mut self, v: &View, model: &Model, p: &pose::Pose, scale: f32, time: f32, flags: u32) -> Option<([f32; 3], [f32; 3])> {
        let mut light = None;
        let t = assets::sprites();
        for (i, n) in model.sod.nodes.iter().enumerate() {
            if !matches!(n.kind, a2fmt::sod::NodeKind::Sprite) || !p.shown.get(i).copied().unwrap_or(false) {
                continue;
            }
            let Some(nd) = t.sprite_node(n.sprite_node_id()) else { continue };
            let Some(sp) = t.sprite(&nd.sprite) else { continue };
            let mut colour = nd.colour;
            if let Some(a) = nd.animation.as_deref().and_then(|a| t.animation(a)) {
                match crate::mods::hulls::anim::value(a, time, flags) {
                    Some(crate::mods::hulls::anim::Value::Draw(false)) => continue,
                    Some(crate::mods::hulls::anim::Value::Colour(c)) => {
                        colour = c;
                        if light.is_none() {
                            let mut peak = c;
                            for k in &a.keys {
                                if let a2fmt::spr::KeyValue::Colour(kc) = k.value {
                                    if kc[0] + kc[1] + kc[2] > peak[0] + peak[1] + peak[2] {
                                        peak = kc;
                                    }
                                }
                            }
                            light = Some((c, peak));
                        }
                    }
                    _ => {}
                }
            }
            if !self.look.nimbus {
                continue;
            }
            let r = sp.uv_rect().unwrap_or_else(|| {
                let rw = sp.reference[0].unwrap_or(256.0).max(1.0);
                let rh = sp.reference[1].unwrap_or(rw).max(1.0);
                [sp.rect[0] / rw, sp.rect[1] / rh, sp.rect[2] / rw, sp.rect[3] / rh]
            });
            let at = p.world[i];
            let pos = at.p;
            let corners = if nd.billboard {
                let vw = &v.view.0;
                let right = v3(vw[0][0], vw[1][0], vw[2][0]);
                let up = v3(vw[0][1], vw[1][1], vw[2][1]);
                let (hx, hy) = (nd.size[0] * scale, nd.size[1] * scale);
                [pos - right * hx + up * hy, pos + right * hx + up * hy, pos + right * hx - up * hy, pos - right * hx - up * hy]
            } else {
                let ax = at.r[0] * (1.0 / at.r[0].len().max(1e-6)) * scale;
                let ay = at.r[1] * (1.0 / at.r[1].len().max(1e-6)) * scale;
                let (hx, hy) = (nd.size[0] * 0.5, nd.size[1] * 0.5);
                [pos - ax * hx + ay * hy, pos + ax * hx + ay * hy, pos + ax * hx - ay * hy, pos - ax * hx - ay * hy]
            };
            let add = matches!(sp.material.as_str(), "additive" | "add_nomipf" | "");
            self.sprites.push(Spr {
                tex: sp.texture.clone(),
                add,
                corners,
                uv: [r[0], r[1], r[0] + r[2], r[1] + r[3]],
                colour: [colour[0], colour[1], colour[2], 1.0],
                depth: v.view.transform_point(pos).z,
            });
        }
        light
    }

    /// An asteroid field's rocks; false leaves the field to the engine.
    pub fn draw_field(&mut self, o: usize, v: &View, surf: &mut Surfaces) -> bool {
        let inst = obj::instance(o);
        let cls = obj::class(o);
        let (arr, n) = (field(inst, 0x98), field(inst, 0x9c));
        let dbs = field(cls, 0x1e0);
        let ndb = field(cls, 0x1fc).min(16);
        if arr == 0 || dbs == 0 || n > 256 {
            return false;
        }
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        let scale = self.scale_sod(obj::odf_name(o));
        let limit = unsafe { rd::<f32>(CFG_CULL_DISTANCE) };
        for i in 0..n {
            let a = arr + i * 0x98;
            let db = field(a, 0x80);
            let Some(k) = (0..ndb).find(|&k| field(dbs, k * 4) == db) else { continue };
            let nm = unsafe { rt::cstr_at(field(ASTEROID_NAMES, k * 4) as *const u8, 32) };
            let world0 = field_m34(a, 0x44);
            // GameObjectInstance::DistanceCull, as the field's RenderInternal asks it:
            // the distance with height counted a quarter, against the culling distance.
            let d = world0.p - v.eye;
            if engine::field_u8(a, 0x84) != 0 || (d.x * d.x + d.y * d.y / 16.0 + d.z * d.z) > limit * limit {
                continue;
            }
            let Some(model) = scene::model(v.dev, nm) else { continue };
            let mut world = world0;
            world.r[0] = world.r[0] * scale;
            world.r[1] = world.r[1] * scale;
            world.r[2] = world.r[2] * scale;
            let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::FROZEN, pose::auto(v.eye));
            self.draw_model(v, &model, &p, None, None, surf);
        }
        self.note(0, "an asteroid field's rocks");
        true
    }

    /// `core`: the light inside a dilithium moon's cracks (`crack_light`), None to leave
    /// the recipe's cut as holes.
    fn draw_model(&mut self, v: &View, model: &Rc<Model>, p: &pose::Pose, glow: Option<V3>, core: Option<V3>, surf: &mut Surfaces) {
        let any = self.info(model, surf).any;
        if !any {
            scene::draw(v, model, p, &scene::Params::default());
            return;
        }
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        let mut plain = false;
        let mut fallback: Vec<usize> = Vec::new();
        for pi in 0..model.parts.len() {
            let part = &model.parts[pi];
            if !p.shown.get(part.node).copied().unwrap_or(false) {
                continue;
            }
            let world = p.world[part.node];
            match self.info[&model.name].kinds[pi] {
                Kind::Plain => plain = true,
                Kind::Glow => {
                    if self.look.dark_shell {
                        // As Lighting.asi drew it (a hull, lit from inside its rock): dark,
                        // opaque. Through the rock's cracks it shows black, and the nimbus
                        // behind the moon only where a crack opens past its edge.
                        if !(self.vs.bind(dev) && self.flat.bind(dev) && self.decl.bind(dev)) {
                            continue;
                        }
                        begin_states(dev);
                        dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else { CULL_CW });
                        dev.vs_const(0, world.to_m4().transpose().as_floats());
                        dev.vs_const(4, &gfx::vp_constants(v));
                        dev.ps_const(0, &[self.look.ambient[0], self.look.ambient[1], self.look.ambient[2], 1.0]);
                        dev.set_stream(0, part.vb.ptr(), 0, 32);
                        dev.set_indices(part.ib.ptr());
                        for g in &part.groups {
                            dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
                        }
                    } else if let Some(c) = glow {
                        self.glows.push(Glow { model: model.clone(), part: pi, world, colour: c });
                    }
                }
                Kind::Surface => {
                    let tex = part.texture.clone().unwrap_or_default();
                    let ext = self.info[&model.name].ext[pi];
                    let Some(s) = surf.get(dev, &tex, v.time) else {
                        // No surface now (Surfaces=0, or the bake failed): the stock texture.
                        fallback.push(pi);
                        plain = true;
                        continue;
                    };
                    if !(self.vs.bind(dev) && self.ps.bind(dev) && self.decl.bind(dev)) {
                        continue;
                    }
                    self.consts(v, &world, part.centre, part.radius);
                    surfaces::bind(dev, s, 0, 2);
                    let w = &world;
                    let ctr = w.transform_point(part.centre);
                    // c28..c30 the axes, c31 the centre, c32 the glow, c33 the relief's
                    // strength, the cut and the mapping, c34 the extents and the gloss,
                    // c35 the light in the cracks and the cut's level, c36 the glow's scale,
                    // the white at a crack's core, the cracks' depth and their light's falloff.
                    let mut k = [0.0f32; 9 * 4];
                    for i in 0..3 {
                        k[i * 4..i * 4 + 3].copy_from_slice(&[w.r[i].x, w.r[i].y, w.r[i].z]);
                    }
                    k[12..15].copy_from_slice(&[ctr.x, ctr.y, ctr.z]);
                    k[16..20].copy_from_slice(&s.draw[0]);
                    k[20..24].copy_from_slice(&[relief_strength(s), 0.0, s.draw[3][2], s.model[0]]);
                    k[24..28].copy_from_slice(&[ext.x, ext.y, ext.z, s.model[1]]);
                    k[32] = 1.0;
                    if let Some(c) = core.filter(|_| s.draw[3][2] > 0.0) {
                        // The cracks lit from inside: no hole, so neither the shell nor the
                        // space behind shows through; the recipe's glow beside them follows
                        // the light (its pulse, the dilithium left).
                        // The recipe's glow keeps its strength against the full light and
                        // takes the light's hue (the infinite moons' is purple).
                        k[28..32].copy_from_slice(&[c.x, c.y, c.z, s.draw[3][3].max(1e-3)]);
                        let g = v3(s.draw[0][0], s.draw[0][1], s.draw[0][2]).len() / (v3(0.0, 0.5, 1.0) * self.look.cracks).len().max(1e-6);
                        k[16..19].copy_from_slice(&[c.x * g, c.y * g, c.z * g]);
                        k[33] = self.look.crack_core;
                        k[34] = self.look.crack_depth;
                        k[35] = self.look.crack_falloff;
                        self.note(4, "a moon's cracks lit from inside");
                    }
                    dev.ps_const(28, &k);
                    begin_states(dev);
                    let mirrored = w.det3() < 0.0;
                    dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else if mirrored { CULL_CCW } else { CULL_CW });
                    dev.vs_const(0, w.to_m4().transpose().as_floats());
                    dev.vs_const(4, &gfx::vp_constants(v));
                    dev.set_stream(0, part.vb.ptr(), 0, 32);
                    dev.set_indices(part.ib.ptr());
                    for g in &part.groups {
                        dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
                    }
                    self.note(1, "a surface from its recipe on a model");
                }
            }
        }
        if plain {
            // The parts without a recipe: the scene's own draw, of those parts only.
            if scene::begin(v) {
                for pi in 0..model.parts.len() {
                    let part = &model.parts[pi];
                    if (self.info[&model.name].kinds[pi] == Kind::Plain || fallback.contains(&pi)) && p.shown.get(part.node).copied().unwrap_or(false) {
                        scene::draw_part(v, model, pi, &p.world[part.node], &scene::Params::default());
                    }
                }
            }
        }
    }

    /// The lights and the look (c0..c27).
    fn consts(&self, v: &View, world: &M34, centre: V3, radius: f32) {
        let mut k = [0.0f32; 28 * 4];
        let ls = scene::lights();
        k[0..3].copy_from_slice(&self.look.ambient);
        k[4..8].copy_from_slice(&[self.look.spec, self.look.spec_pow, self.look.rim_pow, self.look.sun]);
        k[8..11].copy_from_slice(&[v.eye.x, v.eye.y, v.eye.z]);
        k[12..15].copy_from_slice(&self.look.rim);
        let key = ls.key();
        for (i, d) in ls.dirs.iter().take(4).enumerate() {
            let is_key = key.is_some_and(|kd| kd.dir == d.dir && kd.colour == d.colour);
            k[(4 + i) * 4..(4 + i) * 4 + 4].copy_from_slice(&[d.dir.x, d.dir.y, d.dir.z, if is_key { 1.0 } else { 0.0 }]);
            k[(8 + i) * 4..(8 + i) * 4 + 3].copy_from_slice(&[d.colour.x, d.colour.y, d.colour.z]);
        }
        let c = world.transform_point(centre);
        let r = radius * world.r[0].len().max(world.r[1].len()).max(world.r[2].len());
        for (i, p) in ls.points_near(c, r, 8).iter().enumerate() {
            k[(12 + i) * 4..(12 + i) * 4 + 4].copy_from_slice(&[p.pos.x, p.pos.y, p.pos.z, p.start]);
            k[(20 + i) * 4..(20 + i) * 4 + 4].copy_from_slice(&[p.colour.x, p.colour.y, p.colour.z, p.range]);
        }
        v.dev.ps_const(0, &k);
    }

    /// At the translucent step: the glow shells, added, writing no depth.
    pub fn translucent(&mut self, v: &View) {
        self.draw_glows(v);
        self.draw_sprites(v);
    }

    fn draw_sprites(&mut self, v: &View) {
        if self.sprites.is_empty() {
            return;
        }
        let mut sprites = core::mem::take(&mut self.sprites);
        sprites.sort_by(|a, b| b.depth.partial_cmp(&a.depth).unwrap_or(core::cmp::Ordering::Equal));
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.svs.bind(dev) && self.sps.bind(dev) && self.sdecl.bind(dev)) {
            return;
        }
        begin_states(dev);
        dev.set_rs(RS_CULLMODE, CULL_NONE);
        dev.set_rs(RS_ZWRITEENABLE, 0);
        dev.set_rs(RS_ALPHABLENDENABLE, 1);
        dev.vs_const(4, &gfx::vp_constants(v));
        for (st, val) in [(SAMP_ADDRESSU, TADDRESS_WRAP), (SAMP_ADDRESSV, TADDRESS_WRAP), (SAMP_MAGFILTER, TEXF_LINEAR), (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_LINEAR), (SAMP_SRGBTEXTURE, 0)] {
            dev.set_ss(0, st, val);
        }
        for q in &sprites {
            dev.set_rs(RS_SRCBLEND, if q.add { BLEND_ONE } else { BLEND_SRCALPHA });
            dev.set_rs(RS_DESTBLEND, if q.add { BLEND_ONE } else { BLEND_INVSRCALPHA });
            dev.ps_const(0, &[if q.add { 0.0 } else { 1.0 }, 0.0, 0.0, 0.0]);
            dev.set_texture(0, assets::texture(dev, &q.tex).unwrap_or(core::ptr::null_mut()));
            let uv = [[q.uv[0], q.uv[1]], [q.uv[2], q.uv[1]], [q.uv[2], q.uv[3]], [q.uv[0], q.uv[3]]];
            let mk = |k: usize| SVert { pos: [q.corners[k].x, q.corners[k].y, q.corners[k].z], col: q.colour, uv: uv[k] };
            let vs = [mk(0), mk(1), mk(2), mk(0), mk(2), mk(3)];
            if let Some(first) = self.svb.push(dev, &vs, core::mem::size_of::<SVert>() as u32) {
                dev.draw_primitive(PT_TRIANGLELIST, first, 2);
            }
        }
        self.note(3, "a moon's sprite nodes");
    }

    fn draw_glows(&mut self, v: &View) {
        if self.glows.is_empty() {
            return;
        }
        let glows = core::mem::take(&mut self.glows);
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !(self.vs.bind(dev) && self.flat.bind(dev) && self.decl.bind(dev)) {
            return;
        }
        begin_states(dev);
        dev.set_rs(RS_ALPHABLENDENABLE, 1);
        dev.set_rs(RS_SRCBLEND, BLEND_ONE);
        dev.set_rs(RS_DESTBLEND, BLEND_ONE);
        dev.set_rs(RS_ZWRITEENABLE, 0);
        dev.vs_const(4, &gfx::vp_constants(v));
        for g in &glows {
            let part = &g.model.parts[g.part];
            dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else { CULL_CW });
            dev.vs_const(0, g.world.to_m4().transpose().as_floats());
            dev.ps_const(0, &[g.colour.x, g.colour.y, g.colour.z, 1.0]);
            dev.set_stream(0, part.vb.ptr(), 0, 32);
            dev.set_indices(part.ib.ptr());
            for gr in &part.groups {
                dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, gr.start, gr.tris);
            }
        }
        self.note(2, "a moon's glow shell");
    }
}

fn relief_strength(s: &surfaces::Surface) -> f32 {
    // Surfaces.asi's k[3].x, the relief's strength in the light ("Depth="), is the
    // tilt's scale for a model too.
    s.draw[3][0]
}

fn begin_states(dev: Device) {
    for (s, val) in [
        (RS_ZENABLE, 1), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ZWRITEENABLE, 1), (RS_FOGENABLE, 0), (RS_STENCILENABLE, 0),
        (RS_SCISSORTESTENABLE, 0), (RS_COLORWRITEENABLE, 0xf), (RS_FILLMODE, 3), (RS_BLENDOP, BLENDOP_ADD),
        (RS_SEPARATEALPHABLENDENABLE, 0), (RS_SRGBWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 0),
    ] {
        dev.set_rs(s, val);
    }
    dev.set_stream_freq(0, 1);
}

/// The moon's glow colour as `Scrap::Render` sets it, or None for a Scrap without
/// dilithium (its shell keeps the material's colour: the full glow).
fn glow_colour(o: usize) -> Option<V3> {
    let res = field(o, 0x17c);
    let full = v3(0.0, 0.5, 1.0);
    if res == 0 || field_u8(res, 0x18) != 0 {
        return Some(full);
    }
    let rc = field(res, 4);
    if rc == 0 {
        return Some(full);
    }
    let (val, max) = unsafe {
        let f: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_RES_VALUE);
        let g: unsafe extern "thiscall" fn(usize) -> f32 = core::mem::transmute(FN_RES_MAX);
        (f(res), g(rc))
    };
    let frac = if max > 0.0 { val / max } else { 1.0 };
    let _ = field_f32;
    Some(full * frac)
}
