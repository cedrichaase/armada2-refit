//! Sky: the whole of `Background_Render` (body-map.md, 2a) drawn by us. The map's sky
//! (the procedural sky of `Sky.asi`, baked from its recipe, or the stock cube's faces
//! where there is none), the starfield, and the background objects (the suns, the
//! wreck), each in the order and with the far plane stock uses, then the depth is
//! cleared over the view for the scene. Supersedes `Sky.asi`.
pub mod bake;
pub mod recipe;
pub mod sprites;

use super::{Module, Step};
use crate::d3d::*;
use crate::engine::{self, field, field_u8, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Decl, Shader};
use crate::math::{M34, M4, V3, v3};
use crate::scene::{self, pose};
use crate::shaders::sky::*;
use crate::{assets, log, rt};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use bake::{Baker, QUAD_DECL, QuadVert};
use recipe::Recipe;
use sprites::{Batch, SPRITE_DECL, SprInfo, Starfield};

// ---- Armada2.exe ------------------------------------------------------------------

/// The map's background name, lower-cased, as `Starfield_Load_Background_Geometry`
/// keeps it: `<name>.sod` (a cube model) or a bare prefix (six faces `<prefix>0..5`).
const BG_NAME: usize = 0x738538;
/// `g_background_geometry_db`: the sky's database; stock draws the sky only with one.
const G_BG_DB: usize = 0x7720fc;
/// `g_starfield`.
const G_STARFIELD: usize = 0x773d04;
/// `STAR_FAR_CLIP_PLANE` and `FAR_CLIPPING_PLANE` (`ART_CFG.h`; QOL may scale the
/// second in memory): the camera's far plane during the background, and after.
const STAR_FAR: usize = 0x6fcac8;
const FAR_PLANE: usize = 0x6fcab0;
/// `Starfield_Load_Background_Geometry(char*)`, cdecl: a new background (bench only).
const FN_LOAD_BG: usize = 0x590c30;
const FN_SET_FAR: usize = 0x6198a0; // ST3D_Camera::SetFarPlane(float), thiscall
/// `UserProfile*`; its byte +0x84 is edit mode (`Program::Toggle_Edit_Mode`).
const USER_PROFILE: usize = 0x76b5ac;
/// The class's `hidden` flag (`dlight` has it): not drawn outside edit mode.
const CLASS_HIDDEN: usize = 0x19c;
/// Object flags: a background object, drawn here and skipped by the object loop.
const OBJ_BACKGROUND: u32 = 0x10000;
const FN_GO_RENDER: usize = 0x4d54e0; // GameObject::Render, slot 11 of the classes that keep it
const SLOT_RENDER: usize = 11;
const VT_DATABASE: usize = 0x6bc56c; // a plain ST3D_Database
const DEV_CLEAR_DEPTH: usize = 0x2c; // ST3D_Device vtable: ClearDepthBuffer(ST3D_Rect*)
const DEV_FLUSH: usize = 0x38; // ST3D_Device vtable: Flush()

/// The six faces `CreateBackgroundFace` builds for a prefix sky: face `i` shows
/// `<prefix><i>` on a quad at +-100 about the camera, corners in this order with
/// texture coordinates (0,0) (1,0) (1,1) (0,1), as two triangles (0,1,2) (0,2,3).
const PREFIX_FACES: [[[f32; 3]; 4]; 6] = [
    [[-100.0, 100.0, 100.0], [100.0, 100.0, 100.0], [100.0, -100.0, 100.0], [-100.0, -100.0, 100.0]],
    [[100.0, 100.0, 100.0], [100.0, 100.0, -100.0], [100.0, -100.0, -100.0], [100.0, -100.0, 100.0]],
    [[100.0, 100.0, -100.0], [-100.0, 100.0, -100.0], [-100.0, -100.0, -100.0], [100.0, -100.0, -100.0]],
    [[-100.0, 100.0, -100.0], [-100.0, 100.0, 100.0], [-100.0, -100.0, 100.0], [-100.0, -100.0, -100.0]],
    [[-100.0, 100.0, -100.0], [100.0, 100.0, -100.0], [100.0, 100.0, 100.0], [-100.0, 100.0, 100.0]],
    [[-100.0, -100.0, 100.0], [100.0, -100.0, 100.0], [100.0, -100.0, -100.0], [-100.0, -100.0, -100.0]],
];

unsafe fn this0(f: usize, this: usize) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize)>(f)(this) }
}
unsafe fn this1(f: usize, this: usize, a: usize) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, usize)>(f)(this, a) }
}
unsafe fn set_far(cam: usize, far: f32) {
    unsafe { core::mem::transmute::<usize, unsafe extern "thiscall" fn(usize, f32)>(FN_SET_FAR)(cam, far) }
}

/// `Render3D.ini` `[Sky]`.
struct Cfg {
    /// 0: the engine's own `Background_Render` (for comparing on the bench; read live).
    draw: bool,
    /// 0: the stock cube's faces even where a recipe exists.
    recipes: bool,
    /// The baked cube's edge; 0 computes the sky at every pixel (the reference).
    face: u32,
    /// Re-read `Sky\<name>.ini` in the game directory when it changes.
    reload: bool,
    dump: bool,
    /// How far past white the brightest texels of the starfield and of the suns go.
    star_lift: f32,
    sun_lift: f32,
    /// For the bench: a background to load in place of the map's
    /// (`Starfield_Load_Background_Geometry`), once per new value.
    load: String,
}

impl Cfg {
    fn read() -> Cfg {
        let ini = crate::config::ini();
        let face = ini.int("Sky", "Face", 1536).max(0) as u32;
        Cfg {
            draw: ini.int("Sky", "Draw", 1) != 0,
            recipes: ini.int("Sky", "Recipes", 1) != 0,
            face: if face == 0 { 0 } else { face.clamp(64, 4096) },
            reload: ini.int("Sky", "Reload", 1) != 0,
            dump: ini.int("Sky", "Dump", 0) != 0,
            star_lift: ini.float("Sky", "StarLift", 0.5).max(0.0),
            sun_lift: ini.float("Sky", "SunLift", 1.0).max(0.0),
            load: ini.string("Sky", "Load").unwrap_or_default(),
        }
    }
}

pub struct Sky {
    cfg: Cfg,
    baker: Baker,
    /// The background name the recipe below was looked up for.
    bg: String,
    recipe: Option<Recipe>,
    frames: u32,
    stars: Starfield,
    batch: Batch,
    /// Per ODF: its SOD's scale (`ScaleSOD`).
    scales: BTreeMap<String, f32>,
    /// Sprite nodes by id: their sprite, half extents and colour.
    nodes: BTreeMap<String, Option<(SprInfo, [f32; 2], [f32; 3])>>,
    vs_sky: Shader,
    ps_sky: Shader,
    ps_draw: Shader,
    vs_face: Shader,
    ps_face: Shader,
    vs_spr: Shader,
    ps_spr: Shader,
    quad_decl: Decl,
    face_decl: Decl,
    spr_decl: Decl,
    said_inst: bool,
    said_objects: usize,
    tick: u32,
    loaded: String,
}

impl Default for Sky {
    fn default() -> Sky {
        Sky {
            cfg: Cfg::read(),
            baker: Baker::new(),
            bg: String::new(),
            recipe: None,
            frames: 0,
            stars: Starfield::default(),
            batch: Batch::default(),
            scales: BTreeMap::new(),
            nodes: BTreeMap::new(),
            vs_sky: Shader::vertex(&VS_SKY),
            ps_sky: Shader::pixel(&PS_SKY),
            ps_draw: Shader::pixel(&PS_DRAW),
            vs_face: Shader::vertex(&VS_FACE),
            ps_face: Shader::pixel(&PS_FACE),
            vs_spr: Shader::vertex(&VS_SPRITE),
            ps_spr: Shader::pixel(&PS_SPRITE),
            quad_decl: Decl::new(&QUAD_DECL),
            face_decl: Decl::new(&scene::VERTEX),
            spr_decl: Decl::new(&SPRITE_DECL),
            said_inst: false,
            said_objects: 0,
            tick: 0,
            loaded: String::new(),
        }
    }
}

/// The view's projection with its far plane moved out to `far`, as the engine's CPU
/// projection is while it draws the background (the device's keeps the scene's far
/// plane, which would clip the starfield).
fn with_far(proj: &M4, far: f32) -> M4 {
    let mut p = *proj;
    let (a, b) = (p.0[2][2], p.0[3][2]);
    if a.abs() < 1e-9 || p.0[2][3] == 0.0 {
        return p;
    }
    let near = -b / a;
    if !(near > 0.0 && far > near) {
        return p;
    }
    p.0[2][2] = far / (far - near);
    p.0[3][2] = -near * far / (far - near);
    p
}

/// A copy of the view with another projection.
fn view_with(v: &View, proj: M4) -> View {
    View {
        cam: v.cam,
        rs: v.rs,
        main: v.main,
        rect: v.rect,
        dev: v.dev,
        st3d: v.st3d,
        view: v.view,
        proj,
        eye: v.eye,
        time: v.time,
        frame: v.frame,
    }
}

impl Sky {
    /// The recipe for the map's sky now, looked up again when the name changes, and
    /// re-read every 30 frames from the game directory's file when `Reload=1`.
    fn recipe_now(&mut self, bg: &str) -> Option<&Recipe> {
        self.frames = self.frames.wrapping_add(1);
        if bg != self.bg {
            self.bg = String::from(bg);
            let base = recipe::base_name(bg);
            self.recipe = recipe::text_for(&base).map(|(t, src)| Recipe::parse(&base, t, src));
            match &self.recipe {
                Some(r) => log!("sky: \"{}\": drawn from its recipe ({})", bg, r.source),
                None => log!("sky: \"{}\": no recipe, the stock sky", bg),
            }
        } else if self.cfg.reload && self.frames % 30 == 0 {
            let base = recipe::base_name(bg);
            let now = recipe::text_for(&base);
            let changed = match (&now, &self.recipe) {
                (Some((t, _)), Some(r)) => *t != r.text,
                (None, None) => false,
                _ => true,
            };
            if changed {
                self.recipe = now.map(|(t, src)| Recipe::parse(&base, t, src));
                log!("sky: recipe \"{}\" read again", base);
            }
        }
        self.recipe.as_ref()
    }

    /// The procedural sky: a quad over the viewport carrying the world ray of each
    /// corner, sampling the baked cube (or computing every pixel, `Face=0`). False if
    /// it cannot draw (the stock faces are drawn instead).
    fn draw_recipe(&mut self, v: &View, vp: &M4) -> bool {
        let _z = crate::timing::zone("sky: cube");
        let dev = v.dev;
        let Some(inv) = vp.inverse() else { return false };
        let corner = |x: f32, y: f32| -> [f32; 3] {
            let f = inv.transform([x, y, 1.0, 1.0]);
            let n = inv.transform([x, y, 0.0, 1.0]);
            let (f, n) = (v3(f[0] / f[3], f[1] / f[3], f[2] / f[3]), v3(n[0] / n[3], n[1] / n[3], n[2] / n[3]));
            let r = f - n;
            [r.x, r.y, r.z]
        };
        let quad: [QuadVert; 4] = [
            QuadVert { pos: [-1.0, 1.0, 0.0], ray: corner(-1.0, 1.0) },
            QuadVert { pos: [1.0, 1.0, 0.0], ray: corner(1.0, 1.0) },
            QuadVert { pos: [-1.0, -1.0, 0.0], ray: corner(-1.0, -1.0) },
            QuadVert { pos: [1.0, -1.0, 0.0], ray: corner(1.0, -1.0) },
        ];
        if !(self.vs_sky.bind(dev) && self.quad_decl.bind(dev)) {
            return false;
        }
        let edge = self.cfg.face;
        let dump = self.cfg.dump;
        let Some(r) = self.recipe.as_ref() else { return false };
        let direct = edge == 0;
        if !direct && !self.baker.holds(r, edge) && !self.baker.bake(dev, r, edge, dump) {
            return false;
        }
        let ps = if direct { &mut self.ps_sky } else { &mut self.ps_draw };
        if !ps.bind(dev) {
            return false;
        }
        for (st, val) in [
            (RS_ZENABLE, 0), (RS_ZWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 0), (RS_FOGENABLE, 0),
            (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3), (RS_STENCILENABLE, 0), (RS_COLORWRITEENABLE, 0xf),
            (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0), (152, 0),
        ] {
            dev.set_rs(st, val);
        }
        dev.ps_const(0, r.floats());
        if !direct {
            let cube = self.baker.cube.as_ref().map_or(core::ptr::null_mut(), |c| c.ptr());
            dev.set_texture(0, cube);
            for (s, val) in [(SAMP_ADDRESSU, TADDRESS_CLAMP), (SAMP_ADDRESSV, TADDRESS_CLAMP), (SAMP_ADDRESSW, TADDRESS_CLAMP),
                (SAMP_MAGFILTER, TEXF_LINEAR), (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_NONE), (SAMP_SRGBTEXTURE, 0)] {
                dev.set_ss(0, s, val);
            }
            let fine = if r.k[21][0] != 0.0 { self.baker.fine(dev) } else { None };
            match fine {
                Some(f) => {
                    dev.set_texture(1, f);
                    for (s, val) in [(SAMP_ADDRESSU, TADDRESS_WRAP), (SAMP_ADDRESSV, TADDRESS_WRAP), (SAMP_ADDRESSW, TADDRESS_WRAP),
                        (SAMP_MAGFILTER, TEXF_LINEAR), (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_LINEAR), (SAMP_SRGBTEXTURE, 0)] {
                        dev.set_ss(1, s, val);
                    }
                }
                None => {
                    dev.ps_const(21, &[0.0; 4]); // no detail: exp(0)
                }
            }
        }
        dev.draw_primitive_up(PT_TRIANGLESTRIP, 2, quad.as_ptr() as *const u8, 24);
        dev.set_texture(1, core::ptr::null_mut());
        true
    }

    /// The stock sky: the cube SOD's meshes, or the six prefix faces, at the camera,
    /// unlit, depth off.
    fn draw_stock(&mut self, v: &View, vp: &M4, bg: &str) {
        let _z = crate::timing::zone("sky: stock faces");
        let dev = v.dev;
        if !(self.vs_face.bind(dev) && self.ps_face.bind(dev) && self.face_decl.bind(dev)) {
            return;
        }
        for (st, val) in [
            (RS_ZENABLE, 0), (RS_ZWRITEENABLE, 0), (RS_ALPHATESTENABLE, 0), (RS_ALPHABLENDENABLE, 0), (RS_FOGENABLE, 0),
            (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3), (RS_STENCILENABLE, 0), (RS_COLORWRITEENABLE, 0xf),
            (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(st, val);
        }
        // The engine clamps every sky texture (`ClampTexturesRecursive`).
        for (s, val) in [(SAMP_ADDRESSU, TADDRESS_CLAMP), (SAMP_ADDRESSV, TADDRESS_CLAMP), (SAMP_MAGFILTER, TEXF_LINEAR),
            (SAMP_MINFILTER, TEXF_LINEAR), (SAMP_MIPFILTER, TEXF_LINEAR), (SAMP_SRGBTEXTURE, 0)] {
            dev.set_ss(0, s, val);
        }
        dev.ps_const(0, &[1.0, 1.0, 1.0, 0.0]);
        // The background instance sits at the camera; its turn and scale are its own.
        let inst = engine::field_m34(0x772068, 0x44);
        let mut world = M34 { r: inst.r, p: v.eye };
        if !(0.5..2.0).contains(&world.r[0].len()) {
            world.r = M34::IDENTITY.r;
        }
        if !self.said_inst {
            self.said_inst = true;
            log!("sky: background instance axes {:?}", inst.r);
        }
        if bg.contains(".sod") {
            let Some(model) = scene::model(dev, bg) else { return };
            let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::looping(v.time), pose::Lod::Forced(usize::MAX));
            for part in model.parts.iter() {
                if !p.shown.get(part.node).copied().unwrap_or(false) {
                    continue;
                }
                // The engine culls the sky's back faces as any mesh's: `mbg02.sod`
                // holds two cubes, and only the one wound inwards shows.
                let mirrored = p.world[part.node].det3() < 0.0;
                dev.set_rs(RS_CULLMODE, if part.two_sided { CULL_NONE } else if mirrored { CULL_CW } else { CULL_CCW });
                let wvp = p.world[part.node].to_m4().mul(vp);
                dev.vs_const(0, wvp.transpose().as_floats());
                let tex = part.texture.as_deref().and_then(|t| assets::texture(dev, t));
                dev.set_texture(0, tex.unwrap_or(core::ptr::null_mut()));
                dev.ps_const(0, &[1.0, 1.0, 1.0, if tex.is_some() { 0.0 } else { 1.0 }]);
                dev.set_stream(0, part.vb.ptr(), 0, 32);
                dev.set_indices(part.ib.ptr());
                for g in &part.groups {
                    dev.draw_indexed(PT_TRIANGLELIST, 0, 0, part.verts, g.start, g.tris);
                }
            }
        } else {
            let wvp = world.to_m4().mul(vp);
            dev.vs_const(0, wvp.transpose().as_floats());
            dev.set_rs(RS_CULLMODE, CULL_CCW);
            let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            for (i, face) in PREFIX_FACES.iter().enumerate() {
                let mut name = String::from(bg);
                name.push((b'0' + i as u8) as char);
                let tex = assets::texture(dev, &name);
                dev.set_texture(0, tex.unwrap_or(core::ptr::null_mut()));
                dev.ps_const(0, &[1.0, 1.0, 1.0, if tex.is_some() { 0.0 } else { 1.0 }]);
                let vtx: Vec<scene::Vertex> = [0usize, 1, 2, 0, 2, 3]
                    .iter()
                    .map(|&k| scene::Vertex { pos: face[k], nrm: [0.0; 3], uv: uv[k] })
                    .collect();
                dev.draw_primitive_up(PT_TRIANGLELIST, 2, vtx.as_ptr() as *const u8, 32);
            }
        }
    }

    fn scale_sod(&mut self, odf: &str) -> f32 {
        if let Some(s) = self.scales.get(odf) {
            return *s;
        }
        let s = assets::odf(odf).and_then(|o| o.get_f32("ScaleSOD")).filter(|s| *s > 0.0).unwrap_or(1.0);
        self.scales.insert(String::from(odf), s);
        s
    }

    fn sprite_node(&mut self, id: &str) -> Option<(SprInfo, [f32; 2], [f32; 3])> {
        let key = rt::lower(id);
        if let Some(n) = self.nodes.get(&key) {
            return n.clone();
        }
        let t = assets::sprites();
        let n = t.sprite_node(id).and_then(|d| {
            let info = sprites::sprite_info(&d.sprite)?;
            Some((info, d.size, d.colour))
        });
        if n.is_none() {
            log!("sky: sprite node {:?} not found", id);
        }
        self.nodes.insert(key, n.clone());
        n
    }

    /// One background object, in place of its `Render` (`GameObject::Render`: nothing
    /// when expired or unseen, else its SOD). Its meshes are drawn now, lit by the
    /// view's lights; its sprite nodes go into the batch. False: the engine's draw.
    fn object(&mut self, o: usize, v: &View) -> bool {
        let _z = crate::timing::zone("sky: background objects");
        if field(engine::vtable(o), SLOT_RENDER * 4) != FN_GO_RENDER {
            return false;
        }
        let inst = obj::instance(o);
        let db = field(inst, 0x80);
        if db == 0 || engine::vtable(db) != VT_DATABASE {
            return false;
        }
        let name = obj::odf_name(o);
        if name.is_empty() {
            return false;
        }
        let Some(model) = scene::model(v.dev, &crate::assets::sod_of(name)) else { return false };
        if obj::expired(o) || !obj::can_user_see(o) {
            return true;
        }
        let s = if inst == 0 { v3(1.0, 1.0, 1.0) } else { unsafe { rd::<V3>(inst + 0x28) } } * self.scale_sod(name);
        let mut world: M34 = obj::transform(o);
        world.r[0] = world.r[0] * s.x;
        world.r[1] = world.r[1] * s.y;
        world.r[2] = world.r[2] * s.z;
        let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::of_instance(inst, v.time), pose::auto(v.eye));
        self.draw_background(v, &model, &p);
        true
    }

    /// A background object's posed model: its meshes now, lit by the view's lights, and
    /// its sprite nodes into the batch.
    fn draw_background(&mut self, v: &View, model: &scene::Model, p: &pose::Pose) {
        if !model.parts.is_empty() {
            let _g = gfx::Guard::new(v.dev);
            if scene::begin(v) {
                let params = scene::Params::default();
                // Opaque first, then what blends, all now: the background's depth is
                // cleared before the scene, so nothing may wait for the flush.
                for pass in 0..2 {
                    for (pi, part) in model.parts.iter().enumerate() {
                        let blended = !matches!(part.blend, scene::Blend::Opaque | scene::Blend::Cutout);
                        if blended != (pass == 1) || !p.shown.get(part.node).copied().unwrap_or(false) {
                            continue;
                        }
                        scene::draw_part(v, &model, pi, &p.world[part.node], &params);
                    }
                }
            }
        }
        let (right, up) = sprites::camera_axes(&v.view);
        for (i, node) in model.sod.nodes.iter().enumerate() {
            if !matches!(node.kind, a2fmt::sod::NodeKind::Sprite) || !p.shown.get(i).copied().unwrap_or(false) {
                continue;
            }
            let Some((info, size, col)) = self.sprite_node(node.sprite_node_id()) else { continue };
            let w = &p.world[i];
            let k = (w.r[0].len() + w.r[1].len()) * 0.5;
            let lift = self.cfg.sun_lift;
            self.batch.billboard(&info, w.p, right, up, size[0] * k, size[1] * k, [col[0], col[1], col[2], lift]);
        }
    }

    /// The menu scene's sky (`crate::menu`): its recipe turned by the orbit, its stars,
    /// its sun. Drawn about the eye, so the sky's own view has no translation; nothing
    /// writes depth.
    fn menu_sky(&mut self, shot: &crate::menu::Shot, v: &View) {
        let dev = v.dev;
        let far = shot.sun.as_ref().map_or(1.0e4, |s| s.2) * 2.0;
        let mut rot = v.view;
        rot.0[3] = [0.0, 0.0, 0.0, 1.0];
        let a = crate::frame::steps::View { view: shot.sky_turn.to_m4().mul(&rot), proj: with_far(&v.proj, far), eye: V3::ZERO, ..*v };
        let vp = a.view.mul(&a.proj);
        let _g = gfx::Guard::new(dev);
        dev.set_stream_freq(0, 1);
        let r = v.rect;
        let rc = Rect { left: r.x as i32, top: r.y as i32, right: (r.x + r.w) as i32, bottom: (r.y + r.h) as i32 };
        dev.clear(&[rc], CLEAR_TARGET, 0xff00_0000, 1.0, 0);
        if self.cfg.recipes && self.recipe_now(&shot.sky).is_some() {
            self.draw_recipe(&a, &vp);
        }
        // The stars, in the sky's frame.
        let star = |n: &str| sprites::sprite_info(n);
        if let (Some(small), Some(big)) = (star("bstar.0"), star("mstar.0")) {
            let (right, up) = sprites::camera_axes(&a.view);
            let d = far * 0.25;
            let lift = self.cfg.star_lift;
            for &(dir, size, b) in &shot.stars {
                let info = if b > 0.7 { &big } else { &small };
                let h = size * d;
                self.batch.billboard(info, dir * d, right, up, h, h, [b, b, b, if b > 0.7 { lift } else { 0.0 }]);
            }
            self.draw_batch(&a, &vp);
        }
        // The sun: a background object's SOD and its flare, where the light comes from.
        if let Some((odf, dir, dist)) = &shot.sun {
            let b = crate::frame::steps::View { view: rot, ..a };
            let bvp = b.view.mul(&b.proj);
            if let Some(model) = scene::model(dev, &assets::sod_of(odf)) {
                let s = self.scale_sod(odf);
                let mut world = M34::IDENTITY;
                world.r = [world.r[0] * s, world.r[1] * s, world.r[2] * s];
                world.p = *dir * *dist;
                let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::looping(v.time), pose::Lod::Forced(usize::MAX));
                self.draw_background(&b, &model, &p);
                self.draw_batch(&b, &bvp);
            }
        }
    }

    fn draw_batch(&mut self, v: &View, vp: &M4) {
        let _z = crate::timing::zone("sky: sprite batch");
        if self.batch.is_empty() {
            return;
        }
        let dev = v.dev;
        if !(self.vs_spr.bind(dev) && self.ps_spr.bind(dev) && self.spr_decl.bind(dev)) {
            return;
        }
        dev.set_rs(RS_ZENABLE, 1);
        dev.set_rs(RS_ZFUNC, CMP_LESSEQUAL);
        dev.set_rs(RS_FOGENABLE, 0);
        dev.set_rs(RS_STENCILENABLE, 0);
        dev.set_rs(RS_SCISSORTESTENABLE, 0);
        dev.set_rs(RS_COLORWRITEENABLE, 0xf);
        dev.set_rs(RS_SRGBWRITEENABLE, 0);
        self.batch.draw(dev, vp);
    }
}

impl Module for Sky {
    fn name(&self) -> &'static str {
        "sky"
    }
    fn superseded(&self) -> Option<&'static str> {
        Some("Sky.asi")
    }
    fn reset(&mut self) {
        self.baker.reset();
    }
    fn menu(&mut self, pass: crate::menu::Pass, shot: &crate::menu::Shot, v: &View) {
        if pass == crate::menu::Pass::Sky {
            self.menu_sky(shot, v);
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        if s != Step::Background {
            return false;
        }
        let dev = v.dev;
        let cam = v.cam;
        // Render3D.ini's [Sky] is read again every 60 views, for tuning live.
        self.tick = self.tick.wrapping_add(1);
        if self.tick % 60 == 0 {
            self.cfg = Cfg::read();
        }
        if !self.cfg.load.is_empty() && self.cfg.load != self.loaded {
            self.loaded = self.cfg.load.clone();
            log!("sky: Load={}: loading it as the map's background", self.loaded);
            let mut c = rt::cstr(&self.loaded);
            unsafe { core::mem::transmute::<usize, unsafe extern "C" fn(*mut u8)>(FN_LOAD_BG)(c.as_mut_ptr()) };
        }
        if !self.cfg.draw {
            return false;
        }
        let rsf = |o: usize| field(v.rs, o) != 0;
        // Background_Render's own test: detail configuration +0x2c == 2 turns the sky off.
        let cfg3d = field(engine::storm3d(), 0x104);
        let sky_on = rsf(4) && field(cfg3d, 0x2c) != 2;
        let stars_on = rsf(8);
        let star_far: f32 = unsafe { rd(STAR_FAR) };
        unsafe { set_far(cam, star_far) };
        let vf = view_with(v, with_far(&v.proj, star_far));
        let vp = vf.view.mul(&vf.proj);
        {
            let _g = gfx::Guard::new(dev);
            dev.set_stream_freq(0, 1);
            let bg = unsafe { String::from(rt::cstr_at(BG_NAME as *const u8, 64)) };
            let have_db = unsafe { rd::<usize>(G_BG_DB) } != 0;
            if sky_on && have_db {
                let recipe = self.cfg.recipes && self.recipe_now(&bg).is_some();
                if !(recipe && self.draw_recipe(&vf, &vp)) {
                    self.draw_stock(&vf, &vp, &bg);
                }
            } else {
                // The only colour clear of the view stock makes: black where no sky is.
                let r = v.rect;
                let rc = Rect { left: r.x as i32, top: r.y as i32, right: (r.x + r.w) as i32, bottom: (r.y + r.h) as i32 };
                dev.clear(&[rc], CLEAR_TARGET, 0xff00_0000, 1.0, 0);
            }
            if stars_on && unsafe { rd::<usize>(G_STARFIELD) } != 0 {
                let lift = self.cfg.star_lift;
                let _z = crate::timing::zone("sky: stars");
                self.stars.gather(&mut self.batch, lift);
                self.draw_batch(&vf, &vp);
            }
        }
        // The background objects, with depth: each draws itself, or the engine does.
        let _zo = crate::timing::zone("sky: object list");
        let edit = field_u8(unsafe { rd::<usize>(USER_PROFILE) }, 0x84) != 0;
        let mut list: Vec<usize> = Vec::new();
        engine::for_each_in_list(engine::OBJECT_LIST, |o| {
            if o != 0 && obj::flags(o) & OBJ_BACKGROUND != 0 && (edit || field_u8(obj::class(o), CLASS_HIDDEN) == 0) {
                list.push(o);
            }
        });
        if list.len() != self.said_objects {
            self.said_objects = list.len();
            for &o in &list {
                let t = obj::transform(o);
                log!("sky: background object {} at {:.0},{:.0},{:.0}", obj::odf_name(o), t.p.x, t.p.y, t.p.z);
            }
        }
        for o in list {
            if !self.object(o, &vf) {
                unsafe { this1(field(engine::vtable(o), SLOT_RENDER * 4), o, cam) };
            }
        }
        {
            let _g = gfx::Guard::new(dev);
            self.draw_batch(&vf, &vp);
        }
        unsafe {
            let dvt = engine::vtable(v.st3d);
            this0(field(dvt, DEV_FLUSH), v.st3d);
            set_far(cam, rd::<f32>(FAR_PLANE));
            let r: [f32; 4] = rd(cam + 0x1b0);
            this1(field(dvt, DEV_CLEAR_DEPTH), v.st3d, r.as_ptr() as usize);
        }
        true
    }
}
