//! Sky: the whole of `Background_Render` (body-map.md, 2a) drawn by us. The map's sky
//! (the procedural sky of `Sky.asi`, baked from its recipe, or the stock cube's faces
//! where there is none), the starfield, and the background objects (the suns, the
//! wreck), each in the order and with the far plane stock uses, then the depth is
//! cleared over the view for the scene. Supersedes `Sky.asi`.
pub mod bake;
pub mod recipe;
pub mod sprites;

use super::{Module, Step};
use crate::engine::{self, field, field_u8, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Pipes};
use crate::math::{M34, M4, V3, v3};
use crate::scene::{self, pose};
use crate::shaders::sky::*;
use crate::{assets, log, rt};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use bake::{draw_quad, quad_desc, Baker, QuadVert};
use gpu::{
    Address, Blend, BlendEq, BlendOp, Block, Cmd, Code, Cull, Factor, Filter, Format, MipFilter, Pipeline, PipelineDesc,
    Rect, Sampler, Topology, Viewport,
};
use recipe::Recipe;
use sprites::{Batch, SprInfo, Starfield};

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
    pipes: Pipes,
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
            pipes: Pipes::new(),
            said_inst: false,
            said_objects: 0,
            tick: 0,
            loaded: String::new(),
        }
    }
}

/// The pipelines (`Sky::pipe`): the sky from its cube, or computed at every pixel; the
/// stock faces (with a cull); the sprites, the warp flashes; the tunnel.
const P_DRAW: u64 = 0;
const P_DIRECT: u64 = 1;
const P_FACE: u64 = 2;
const P_SPRITE: u64 = 3;
const P_FLASH: u64 = 4;
const P_TUNNEL: u64 = 5;

/// The stock faces' textures: clamped (the engine clamps every sky texture,
/// `ClampTexturesRecursive`), filtered through their levels.
const FACE_SAMPLER: Sampler = gfx::sampler(Filter::Linear, MipFilter::Linear, Address::Clamp, Address::Clamp);
/// The baked cube: clamped, filtered, one level.
const CUBE_SAMPLER: Sampler = gfx::sampler(Filter::Linear, MipFilter::None, Address::Clamp, Address::Clamp);
/// The fine detail's volume: wrapping, filtered through its chain.
const FINE_SAMPLER: Sampler = Sampler {
    min: Filter::Linear,
    mag: Filter::Linear,
    mip: MipFilter::Linear,
    address_u: Address::Wrap,
    address_v: Address::Wrap,
    address_w: Address::Wrap,
    anisotropy: 1,
    srgb: false,
};

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
        st3d: v.st3d,
        view: v.view,
        proj,
        eye: v.eye,
        time: v.time,
        frame: v.frame,
    }
}

impl Sky {
    /// The pipeline `which` (`P_*`), with `cull` for the stock faces. All draw into the
    /// view: the sky and its faces with no depth, no blending; the sprites added and
    /// depth-tested; the tunnel over what is there (premultiplied).
    fn pipe(&mut self, which: u64, cull: Cull) -> Option<Pipeline> {
        self.pipes.view(which * 4 + cull as u64, || match which {
            P_DRAW => quad_desc(&PS_DRAW, Format::Unknown),
            P_DIRECT => quad_desc(&PS_SKY, Format::Unknown),
            P_FACE => {
                let mut d = PipelineDesc::new(Code::d3d9(&VS_FACE), Code::d3d9(&PS_FACE), &scene::VERTEX, 32, Topology::TriangleList, Format::Unknown);
                d.cull = cull;
                d
            }
            P_SPRITE => sprites::sprite_desc(&PS_SPRITE),
            P_FLASH => sprites::sprite_desc(&PS_FLASH),
            _ => {
                let mut d = quad_desc(&PS_TUNNEL, Format::Unknown);
                d.blend = Some(Blend { color: BlendEq { src: Factor::One, dst: Factor::InvSrcAlpha, op: BlendOp::Add }, alpha: None });
                d
            }
        })
    }

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
    /// it cannot draw (the stock faces are drawn instead). A bake draws into targets of
    /// its own; `v`'s pass is begun again after it, with the engine's viewport `evp`.
    fn draw_recipe(&mut self, cmd: &mut Cmd, v: &View, vp: &M4, evp: Option<Viewport>) -> bool {
        let _z = crate::timing::zone("sky: cube");
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
        let edge = self.cfg.face;
        let dump = self.cfg.dump;
        let direct = edge == 0;
        let Some(pipe) = self.pipe(if direct { P_DIRECT } else { P_DRAW }, Cull::None) else { return false };
        let Some(r) = self.recipe.as_ref() else { return false };
        if !direct && !self.baker.holds(r, edge) {
            let ok = self.baker.bake(cmd, r, edge, dump);
            gfx::view_pass(cmd, v, evp);
            if !ok {
                return false;
            }
        }
        if !cmd.set_pipeline(&pipe) {
            return false;
        }
        cmd.ps_block(Block::new(0, r.floats()));
        if !direct {
            cmd.bind_texture(0, self.baker.cube.as_ref(), &CUBE_SAMPLER);
            let fine = if r.k[21][0] != 0.0 { self.baker.fine() } else { None };
            match fine {
                Some(f) => cmd.bind_texture(1, Some(&f), &FINE_SAMPLER),
                None => cmd.ps_block(Block::new(21, &[0.0; 4])), // no detail: exp(0)
            }
        }
        draw_quad(cmd, &quad);
        cmd.bind_texture(1, None, &FINE_SAMPLER);
        true
    }

    /// The stock sky: the cube SOD's meshes, or the six prefix faces, at the camera,
    /// unlit, depth off.
    fn draw_stock(&mut self, cmd: &mut Cmd, v: &View, vp: &M4, bg: &str) {
        let _z = crate::timing::zone("sky: stock faces");
        if !self.pipe(P_FACE, Cull::None).is_some_and(|p| p.ready(gfx::gpu())) {
            return;
        }
        cmd.ps_block(Block::new(0, &[1.0, 1.0, 1.0, 0.0]));
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
            let Some(model) = scene::model(bg) else { return };
            let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::looping(v.time), pose::Lod::Forced(usize::MAX));
            for part in model.parts.iter() {
                if !p.shown.get(part.node).copied().unwrap_or(false) {
                    continue;
                }
                // The engine culls the sky's back faces as any mesh's: `mbg02.sod`
                // holds two cubes, and only the one wound inwards shows.
                let mirrored = p.world[part.node].det3() < 0.0;
                let cull = if part.two_sided { Cull::None } else if mirrored { Cull::Clockwise } else { Cull::CounterClockwise };
                let Some(pipe) = self.pipe(P_FACE, cull) else { continue };
                cmd.set_pipeline(&pipe);
                let wvp = p.world[part.node].to_m4().mul(vp);
                cmd.vs_block(Block::new(0, wvp.transpose().as_floats()));
                let tex = part.texture.as_deref().and_then(assets::texture);
                cmd.bind_texture(0, tex.as_ref(), &FACE_SAMPLER);
                cmd.ps_block(Block::new(0, &[1.0, 1.0, 1.0, if tex.is_some() { 0.0 } else { 1.0 }]));
                for g in &part.groups {
                    part.draw(cmd, g.start, g.tris);
                }
            }
        } else {
            let Some(pipe) = self.pipe(P_FACE, Cull::CounterClockwise) else { return };
            cmd.set_pipeline(&pipe);
            let wvp = world.to_m4().mul(vp);
            cmd.vs_block(Block::new(0, wvp.transpose().as_floats()));
            let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
            for (i, face) in PREFIX_FACES.iter().enumerate() {
                let mut name = String::from(bg);
                name.push((b'0' + i as u8) as char);
                let tex = assets::texture(&name);
                cmd.bind_texture(0, tex.as_ref(), &FACE_SAMPLER);
                cmd.ps_block(Block::new(0, &[1.0, 1.0, 1.0, if tex.is_some() { 0.0 } else { 1.0 }]));
                let vtx: Vec<scene::Vertex> = [0usize, 1, 2, 0, 2, 3]
                    .iter()
                    .map(|&k| scene::Vertex { pos: face[k], nrm: [0.0; 3], uv: uv[k] })
                    .collect();
                if let Some(s) = cmd.vertices(&vtx) {
                    cmd.draw(&s);
                }
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
        let Some(model) = scene::model(&crate::assets::sod_of(name)) else { return false };
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
            if let Some(mut cmd) = gfx::cmd() {
                if scene::begin(&mut cmd, v) {
                    let params = scene::Params::default();
                    // Opaque first, then what blends, all now: the background's depth is
                    // cleared before the scene, so nothing may wait for the flush.
                    for pass in 0..2 {
                        for (pi, part) in model.parts.iter().enumerate() {
                            let blended = !matches!(part.blend, scene::Blend::Opaque | scene::Blend::Cutout);
                            if blended != (pass == 1) || !p.shown.get(part.node).copied().unwrap_or(false) {
                                continue;
                            }
                            scene::draw_part(&mut cmd, v, model, pi, &p.world[part.node], &params);
                        }
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

    /// The only colour clear of the view stock makes: black over the view where no sky
    /// is; the engine's viewport `evp` is put back after.
    fn clear(cmd: &mut Cmd, v: &View, evp: Option<Viewport>) {
        let r = v.rect;
        cmd.clear_rects(&[Rect::new(r.x, r.y, r.w, r.h)], Some([0.0, 0.0, 0.0, 1.0]), None);
        if let Some(vp) = evp {
            cmd.set_viewport(&vp);
        }
    }

    /// The menu scene's sky (`crate::menu`): its recipe turned by the orbit, its stars,
    /// its sun. Drawn about the eye, so the sky's own view has no translation; nothing
    /// writes depth.
    fn menu_sky(&mut self, shot: &crate::menu::Shot, v: &View) {
        let far = shot.sun.as_ref().map_or(1.0e4, |s| s.2) * 2.0;
        let mut rot = v.view;
        rot.0[3] = [0.0, 0.0, 0.0, 1.0];
        let a = crate::frame::steps::View { view: shot.sky_turn.to_m4().mul(&rot), proj: with_far(&v.proj, far), eye: V3::ZERO, ..*v };
        let vp = a.view.mul(&a.proj);
        {
            let Some(mut cmd) = gfx::cmd() else { return };
            let evp = gfx::gpu().engine_viewport();
            Self::clear(&mut cmd, v, evp);
            if self.cfg.recipes && self.recipe_now(&shot.sky).is_some() {
                self.draw_recipe(&mut cmd, &a, &vp, evp);
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
                self.draw_batch(&mut cmd, &vp);
            }
        }
        // The sun: a background object's SOD and its flare, where the light comes from.
        if let Some((odf, dir, dist)) = &shot.sun {
            let b = crate::frame::steps::View { view: rot, ..a };
            let bvp = b.view.mul(&b.proj);
            if let Some(model) = scene::model(&assets::sod_of(odf)) {
                let s = self.scale_sod(odf);
                let mut world = M34::IDENTITY;
                world.r = [world.r[0] * s, world.r[1] * s, world.r[2] * s];
                world.p = *dir * *dist;
                let p = pose::pose(&model.sod, &model.info, &world, pose::Clock::looping(v.time), pose::Lod::Forced(usize::MAX));
                self.draw_background(&b, &model, &p);
                if let Some(mut cmd) = gfx::cmd() {
                    self.draw_batch(&mut cmd, &bvp);
                }
            }
        }
    }

    /// A departure from the menu scene (`Render3D_MenuWarp`): the flashes of the ships
    /// gone to warp (the stock `warp_sprite`, depth-tested against the hulls), then
    /// the tunnel over the whole view (`tunnel_ps`), which darkens what is under it.
    fn menu_warp(&mut self, shot: &crate::menu::Shot, v: &View) {
        let vp = v.view.mul(&v.proj);
        let flashes = crate::menu::take_flashes();
        if !flashes.is_empty()
            && let Some(info) = sprites::sprite_info("warp_sprite")
        {
            let (right, up) = sprites::camera_axes(&v.view);
            for (at, half, bright) in flashes {
                self.batch.billboard(&info, at, right, up, half, half, [bright, bright, bright, 1.0]);
            }
            // The sprite shader with the fade to its edge (`flash_ps`): the art's halo
            // reaches its border, which showed as a box at the menu's brightness.
            if let (Some(mut cmd), Some(pipe)) = (gfx::cmd(), self.pipe(P_FLASH, Cull::None)) {
                if pipe.ready(gfx::gpu()) {
                    cmd.ps_block(Block::new(32, &info.uv));
                    self.batch.draw(&mut cmd, &pipe, &vp);
                }
            }
        }
        let Some(tn) = shot.tunnel else { return };
        let Some(inv) = vp.inverse() else { return };
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
        let Some(mut cmd) = gfx::cmd() else { return };
        let Some(pipe) = self.pipe(P_TUNNEL, Cull::None) else { return };
        if !cmd.set_pipeline(&pipe) {
            return;
        }
        // The camera's axes in world space: the rows of the view's rotation, transposed.
        let m = &v.view.0;
        let right = v3(m[0][0], m[1][0], m[2][0]).norm();
        let up = v3(m[0][1], m[1][1], m[2][1]).norm();
        let fwd = v3(m[0][2], m[1][2], m[2][2]).norm();
        let c = tn.colour;
        let k: [f32; 20] = [
            tn.time, tn.k, tn.dim, tn.speed,
            fwd.x, fwd.y, fwd.z, 0.0,
            right.x, right.y, right.z, 0.0,
            up.x, up.y, up.z, 0.0,
            c.x, c.y, c.z, 0.0,
        ];
        cmd.ps_block(Block::new(27, &k));
        draw_quad(&mut cmd, &quad);
    }

    /// The batch's sprites, added and depth-tested.
    fn draw_batch(&mut self, cmd: &mut Cmd, vp: &M4) {
        let _z = crate::timing::zone("sky: sprite batch");
        if self.batch.is_empty() {
            return;
        }
        let Some(pipe) = self.pipe(P_SPRITE, Cull::None) else { return };
        if !pipe.ready(gfx::gpu()) {
            return;
        }
        self.batch.draw(cmd, &pipe, vp);
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
    fn forget(&mut self) {
        self.baker.reset();
        self.baker.fine = None;
    }
    fn menu(&mut self, pass: crate::menu::Pass, shot: &crate::menu::Shot, v: &View) {
        match pass {
            crate::menu::Pass::Sky => self.menu_sky(shot, v),
            crate::menu::Pass::Warp => self.menu_warp(shot, v),
            _ => {}
        }
    }
    fn step(&mut self, s: Step, v: &View) -> bool {
        if s != Step::Background {
            return false;
        }
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
        if let Some(mut cmd) = gfx::cmd() {
            let cmd = &mut cmd;
            let evp = gfx::gpu().engine_viewport();
            let bg = unsafe { String::from(rt::cstr_at(BG_NAME as *const u8, 64)) };
            let have_db = unsafe { rd::<usize>(G_BG_DB) } != 0;
            if sky_on && have_db {
                let recipe = self.cfg.recipes && self.recipe_now(&bg).is_some();
                if !(recipe && self.draw_recipe(cmd, &vf, &vp, evp)) {
                    self.draw_stock(cmd, &vf, &vp, &bg);
                }
            } else {
                Self::clear(cmd, v, evp);
            }
            if stars_on && unsafe { rd::<usize>(G_STARFIELD) } != 0 {
                let lift = self.cfg.star_lift;
                let _z = crate::timing::zone("sky: stars");
                self.stars.gather(&mut self.batch, lift);
                self.draw_batch(cmd, &vp);
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
        if let Some(mut cmd) = gfx::cmd() {
            self.draw_batch(&mut cmd, &vp);
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
