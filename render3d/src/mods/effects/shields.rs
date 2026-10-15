//! Shield hits, ported from `Shields.asi` (shields/README.md, which holds the look
//! settled with the user and every engine fact below): each struck ship's shield
//! ellipsoid drawn by a shader about its hits (hot spot, ring, glow and rim, by
//! faction, weaker as the shields run out), the shields-down ring as jagged arcs that
//! crackle without turning, the special weapons' shells that have a recipe, and the
//! light of the shields-down discharges on the hull.
//!
//! `ShieldEffect::Render` (0x474910) walks `m_shieldList` (0x736de8, a
//! `std::map<int, auto_ptr<ShieldHit>>`: head node at +4 of the map, a node's key at
//! +0xc and its `ShieldHit` at +0x14, `_Nil` at 0x736e38) in order and calls
//! `ShieldHit::DrawShield(cam, [0x736e2c])` (0x474070) for each. We do the same for
//! every hit that is not ours. A hit is ours when its model (`ShieldHit`+0x7c) is type
//! 0's (`m_logical_db`, 0x736ddc, `ST3D_Database *[8]`), type 1's (shields down) or a
//! special weapon's shell with a `[Shell.<name>]` recipe; the names are the engine's
//! table at 0x6b00e0, which `ShieldEffect::GetShieldType` (0x474720) reads.
//!
//! The hit's strength comes from the weapon's damage: the four weapon calls of
//! `ShieldEffect::CreateShieldHit` (0x58bb95 beam, 0x58cad8 shot, 0x58d502 mine,
//! 0x58dd65 missile; the ordnance in `ebx` at each) go through a thunk that notes the
//! ordnance, and the wrapper records the id the call returns against the ordnance
//! class's `damageBase` (class `+0x34`, its `cLookup` at `+0xa8`, the default at
//! `+0x14` in it). Chained: whatever the call went to is called.
//!
//! The clock is the sim clock (`View::time`): the effect stops while the game is
//! paused, as every cosmetic simulation of render3d does.
use super::{object_of, rnd, Cam, LightPool, Sec};
use crate::d3d::*;
use crate::engine::{self, field, field_u8, hook, obj, rd};
use crate::frame::steps::View;
use crate::gfx::{self, Shader};
use crate::log;
use crate::math::{v3, V3};
use crate::shaders::effects_shields::*;
use alloc::string::String;
use alloc::vec::Vec;

const SHIELD_LIST: usize = 0x736de8;
const SHIELD_NIL: usize = 0x736e38;
const SHIELD_LOGICAL_DB: usize = 0x736ddc;
const SHIELD_MATERIAL: usize = 0x736e2c; // what ShieldEffect::Render hands DrawShield
const FN_DRAWSHIELD: usize = 0x474070;
const FN_CREATE_HIT: usize = 0x4743b0;
const SHIELD_TYPE_NAMES: usize = 0x6b00e0;
const FN_GET_SHIELD_TYPE: usize = 0x474720;
const HIT_SITES: [usize; 4] = [0x58bb95, 0x58cad8, 0x58d502, 0x58dd65];
const SITE_NAME: [&str; 4] = ["beam", "shot", "mine", "missile"];

const HIT_MODEL: usize = 0x7c;
const HIT_MODEL_RADIUS: usize = 0x40;
const HIT_SCALE: usize = 0x28;
const HIT_LEFT: usize = 0x84;
const HIT_TIMED: usize = 0x88;
const HIT_HANDLE: usize = 0x8c;
const HIT_XFORM: usize = 0x90;
const HIT_SHIELDS: usize = 0xc0;

const NFAC: usize = 7;
const FAC_KEY: [&str; NFAC] = ["Federation", "Klingon", "Romulan", "Borg", "Cardassian", "Species8472", "Other"];
const FAC_DEF: [[f32; 3]; NFAC] =
    [[90.0, 170.0, 255.0], [255.0, 140.0, 60.0], [90.0, 255.0, 130.0], [110.0, 255.0, 120.0], [255.0, 200.0, 80.0], [210.0, 120.0, 255.0], [140.0, 200.0, 255.0]];

#[derive(Clone, Copy, PartialEq)]
enum ColMode {
    Faction,
    Stock,
    Fixed,
}

struct Set {
    colour_mode: ColMode,
    pattern: f32,
    brightness: f32,
    spot: f32,
    spot_size: f32,
    flash: f32,
    flash_time: f32,
    duration: f32,
    sustain: f32,
    ring: f32,
    ring_speed: f32,
    ring_width: f32,
    ring_time: f32,
    ring_period: f32,
    ring_lattice: f32,
    shells: bool,
    lattice: f32,
    glow_reach: f32,
    glow_time: f32,
    glow_sustain: f32,
    rim: f32,
    rim_power: f32,
    core: f32,
    hex_cells: f32,
    hex_line: f32,
    crackle: f32,
    damage_ref: f32,
    strength_min: f32,
    strength_max: f32,
    fade_out: f32,
    weak_level: f32,
    flicker: f32,
    flicker_below: f32,
    flicker_rate: f32,
    low_shift: f32,
    fac: [V3; NFAC],
    fixed: V3,
    low: V3,
    down: bool,
    down_strikes: usize,
    down_bright: f32,
    down_arc: V3,
    down_glow: V3,
    down_haze: f32,
    down_scale: f32,
    down_warp: f32,
    down_core: f32,
    down_glow_width: f32,
    down_branches: f32,
    down_band: f32,
    down_crackle: f32,
    down_flash: f32,
    down_flash_time: f32,
    down_fade: f32,
    down_size: f32,
    down_time_min: f32,
    down_time_max: f32,
    down_gap: f32,
    down_rate: f32,
    down_far: f32,
    down_rim_power: f32,
    down_lit: f32,
    down_light: f32,
    down_light_col: V3,
    down_light_reach: f32,
}

impl Set {
    fn read(c: &Sec) -> Set {
        let mode = c.s("ColourMode").unwrap_or_default().to_ascii_lowercase();
        let pattern = c.s("Pattern").unwrap_or_default().to_ascii_lowercase();
        let mut fac = [V3::ZERO; NFAC];
        for i in 0..NFAC {
            fac[i] = c.rgb(FAC_KEY[i], FAC_DEF[i]);
        }
        let down_time_min = c.f("DownStrikeMin", 0.12).max(0.02);
        Set {
            colour_mode: if mode.starts_with('s') { ColMode::Stock } else if mode.starts_with("fi") { ColMode::Fixed } else { ColMode::Faction },
            pattern: if pattern.starts_with('c') { 2.0 } else if pattern.starts_with('h') { 1.0 } else { 0.0 },
            brightness: c.f("Brightness", 1.0),
            spot: c.f("Spot", 1.0),
            spot_size: c.f("SpotSize", 0.18).max(0.01),
            flash: c.f("Flash", 1.5),
            flash_time: c.f("FlashTime", 0.08).max(0.005),
            duration: c.f("Duration", 0.45).max(0.01),
            sustain: c.f("Sustain", 0.55).clamp(0.0, 1.0),
            ring: c.f("Ring", 1.1),
            ring_speed: c.f("RingSpeed", 2.4),
            ring_width: c.f("RingWidth", 0.09).max(0.005),
            ring_time: c.f("RingTime", 0.55).max(0.01),
            ring_period: c.f("RingPeriod", 0.4).max(0.05),
            ring_lattice: c.f("RingLattice", 0.0),
            shells: c.i("Shells", 1) != 0,
            lattice: c.f("Lattice", 0.25),
            glow_reach: c.f("GlowReach", 0.35).max(0.01),
            glow_time: c.f("GlowTime", 0.6).max(0.01),
            glow_sustain: c.f("GlowSustain", 0.3).clamp(0.0, 1.0),
            rim: c.f("Rim", 0.8),
            rim_power: c.f("RimPower", 3.0).max(0.25),
            core: c.f("Core", 0.35),
            hex_cells: c.f("HexCells", 7.0).max(1.0),
            hex_line: c.f("HexLine", 0.06).max(0.001),
            crackle: c.f("Crackle", 0.5).clamp(0.0, 1.0),
            damage_ref: c.f("DamageRef", 60.0).max(0.01),
            strength_min: c.f("StrengthMin", 0.6),
            strength_max: c.f("StrengthMax", 1.8),
            fade_out: c.f("FadeOut", 0.25).max(0.01),
            weak_level: c.f("WeakLevel", 0.45).clamp(0.0, 1.0),
            flicker: c.f("Flicker", 0.6).clamp(0.0, 1.0),
            flicker_below: c.f("FlickerBelow", 0.5).clamp(0.01, 1.0),
            flicker_rate: c.f("FlickerRate", 18.0),
            low_shift: c.f("LowShift", 0.6).clamp(0.0, 1.0),
            fac,
            fixed: c.rgb("Colour", [120.0, 190.0, 255.0]),
            low: c.rgb("LowColour", [255.0, 70.0, 40.0]),
            down: c.i("Down", 1) != 0,
            down_strikes: c.i("DownStrikes", 4).clamp(0, 6) as usize,
            down_bright: c.f("DownBrightness", 2.5),
            down_arc: c.rgb("DownArc", [215.0, 230.0, 255.0]),
            down_glow: c.rgb("DownGlow", [140.0, 210.0, 510.0]),
            down_haze: c.f("DownHaze", 0.02),
            down_scale: c.f("DownScale", 2.6).max(0.1),
            down_warp: c.f("DownWarp", 0.55),
            down_core: c.f("DownCore", 2.0).max(0.1),
            down_glow_width: c.f("DownGlowWidth", 0.08).max(0.001),
            down_branches: c.f("DownBranches", 0.6),
            down_band: c.f("DownBand", 0.35).max(0.02),
            down_crackle: c.f("DownCrackle", 0.15),
            down_flash: c.f("DownFlash", 0.6),
            down_flash_time: c.f("DownFlashTime", 0.35).max(0.01),
            down_fade: c.f("DownFadeOut", 0.4).max(0.01),
            down_size: c.f("DownStrikeSize", 0.7).max(0.02),
            down_time_min,
            down_time_max: c.f("DownStrikeMax", 0.45).max(down_time_min),
            down_gap: c.f("DownStrikeGap", 0.15).max(0.0),
            down_rate: c.f("DownRate", 7.0).max(0.1),
            down_far: c.f("DownFarSide", 0.45).clamp(0.0, 1.0),
            down_rim_power: c.f("DownRimPower", 2.5).max(0.25),
            down_lit: c.f("DownLit", -0.2).clamp(-1.0, 1.0),
            down_light: c.f("DownLight", 3.0),
            down_light_col: c.rgb("DownLightColour", [150.0, 180.0, 255.0]),
            down_light_reach: c.f("DownLightReach", 1.2),
        }
    }
}

#[derive(Clone, Default)]
struct Shell {
    model: usize,
    name: String,
    on: bool,
    col: V3,
    bright: f32,
    lattice: f32,
    spot: f32,
    ring: f32,
}

#[derive(Clone, Copy)]
struct Hit {
    id: i32,
    handle: i32,
    /// 0 a beam (held while the engine keeps it), else a single blow.
    site: usize,
    /// 0 the ordinary hit, 2..7 a special weapon's shell.
    ty: usize,
    whole: bool,
    dir: V3,
    strength: f32,
    birth: f64,
    death: f64,
    seen: bool,
}

#[derive(Clone, Copy, Default)]
struct Strike {
    dir: V3,
    t0: f64,
    dur: f64,
    peak: f32,
    generation: u32,
}

#[derive(Clone, Copy)]
struct Down {
    handle: i32,
    seen: bool,
    birth: f64,
    death: f64,
    st: [Strike; 6],
}

// ---- the weapons: which one made a hit -------------------------------------------

#[derive(Clone, Copy)]
struct Made {
    id: i32,
    site: usize,
    damage: f32,
}

const NMADE: usize = 64;
static mut MADE: [Made; NMADE] = [Made { id: -1, site: 0, damage: -1.0 }; NMADE];
static mut MADE_NEXT: usize = 0;
static mut PREV_CREATE: [usize; 4] = [0; 4];
static mut SH_ORD: usize = 0;
static mut SH_SITE: usize = 0;

unsafe extern "C" fn shield_hit(o: usize, m: *const f32, ty: i32, dur: f32, flags: i32) -> i32 {
    unsafe {
        let (site, ord) = (SH_SITE, SH_ORD);
        if site > 3 || PREV_CREATE[site] == 0 {
            return -1;
        }
        let f: unsafe extern "C" fn(usize, *const f32, i32, f32, i32) -> i32 = core::mem::transmute(PREV_CREATE[site]);
        let id = f(o, m, ty, dur, flags);
        if id >= 0 && ord != 0 {
            let cls = field(ord, 0x34);
            let mut dmg = -1.0f32;
            if cls != 0 {
                dmg = rd::<f32>(cls + 0xa8 + 0x14);
                if !(dmg >= 0.0 && dmg < 100000.0) {
                    dmg = -1.0;
                }
            }
            MADE[MADE_NEXT] = Made { id, site, damage: dmg };
            MADE_NEXT = (MADE_NEXT + 1) % NMADE;
        }
        id
    }
}

macro_rules! thunk {
    ($name:ident, $n:literal) => {
        #[unsafe(naked)]
        unsafe extern "C" fn $name() {
            core::arch::naked_asm!(
                "mov dword ptr [{ord}], ebx",
                concat!("mov dword ptr [{site}], ", $n),
                "jmp {f}",
                ord = sym SH_ORD,
                site = sym SH_SITE,
                f = sym shield_hit,
            )
        }
    };
}
thunk!(thunk0, 0);
thunk!(thunk1, 1);
thunk!(thunk2, 2);
thunk!(thunk3, 3);

fn made_of(id: i32) -> Option<Made> {
    unsafe { (*core::ptr::addr_of!(MADE)).iter().copied().find(|m| m.id == id) }
}

// ---- the module part ---------------------------------------------------------------

pub struct Shields {
    pub on: bool,
    s: Set,
    shells: Vec<Shell>,
    shells_db: usize,
    hits: Vec<Hit>,
    downs: Vec<Down>,
    sphere: Vec<[f32; 3]>,
    vs: Shader,
    ps: Shader,
    dps: Shader,
    lights: LightPool,
    /// The last frame could draw the shields-down arcs: stock's ring stands down.
    down_ok: bool,
    logged_hits: u32,
    logged_downs: u32,
    said_first: bool,
    ini: Sec,
}

impl Shields {
    pub fn new() -> Shields {
        let c = Sec::new("Shields");
        Shields {
            on: c.i("Enable", 1) != 0,
            s: Set::read(&c),
            shells: Vec::new(),
            shells_db: usize::MAX,
            hits: Vec::new(),
            downs: Vec::new(),
            sphere: make_sphere(),
            vs: Shader::vertex(&VS_SHIELD),
            ps: Shader::pixel(&PS_SHIELD),
            dps: Shader::pixel(&PS_DOWN),
            lights: LightPool::new(b"shieldsdown\0"),
            down_ok: true,
            logged_hits: 0,
            logged_downs: 0,
            said_first: false,
            ini: c,
        }
    }

    /// The four weapon calls of `CreateShieldHit`, chained through the thunks.
    pub fn install(&mut self) {
        let thunks: [usize; 4] = [thunk0 as *const () as usize, thunk1 as *const () as usize, thunk2 as *const () as usize, thunk3 as *const () as usize];
        let mut names = String::new();
        for i in 0..4 {
            let Some(prev) = hook::call_target(HIT_SITES[i]) else { continue };
            if hook::patch_call(HIT_SITES[i], prev, thunks[i]) {
                unsafe { PREV_CREATE[i] = prev };
                names.push(' ');
                names.push_str(SITE_NAME[i]);
                if prev != FN_CREATE_HIT {
                    names.push_str("(chained)");
                }
            }
        }
        log!("effects: shields: the weapons' hits noted at:{}", names);
    }

    fn db(k: usize) -> usize {
        let db = unsafe { rd::<usize>(SHIELD_LOGICAL_DB) };
        if db == 0 { 0 } else { field(db, k * 4) }
    }

    /// The special weapons' shells: their names from the engine's table, their recipes
    /// from `[Shell.<name>]`.
    fn read_shells(&mut self) {
        let names_ok = unsafe { rd::<usize>(FN_GET_SHIELD_TYPE + 4) } == SHIELD_TYPE_NAMES;
        self.shells = (0..8).map(|_| Shell::default()).collect();
        self.shells_db = Self::db(2);
        let mut said = String::new();
        let mut table_end = false;
        for k in 0..8 {
            let model = Self::db(k);
            self.shells[k].model = model;
            if !names_ok || table_end {
                continue;
            }
            let p = unsafe { rd::<usize>(SHIELD_TYPE_NAMES + k * 4) };
            if p == 0 {
                table_end = true;
                continue;
            }
            if k < 2 || model == 0 {
                continue;
            }
            let name = String::from(unsafe { crate::rt::cstr_at(p as *const u8, 31) });
            let mut sect = String::from("Shell.");
            sect.push_str(&name);
            let ini = &self.ini.1;
            let on = ini.int(&sect, "On", default_shell_on(&name) as i32) != 0 && self.s.shells;
            let (dc, db, dl, ds, dr) = default_shell(&name, &self.s);
            let col = ini.floats(&sect, "Colour", dc);
            let sh = &mut self.shells[k];
            sh.on = on;
            sh.col = v3(col[0] / 255.0, col[1] / 255.0, col[2] / 255.0);
            sh.bright = ini.float(&sect, "Brightness", db);
            sh.lattice = ini.float(&sect, "Lattice", dl);
            sh.spot = ini.float(&sect, "Spot", ds);
            sh.ring = ini.float(&sect, "Ring", dr);
            said.push(' ');
            said.push_str(&name);
            if on {
                said.push_str(" (ours)");
            }
            sh.name = name;
        }
        log!("effects: shields: special weapons' shells:{}", said);
    }

    /// 0 the ordinary hit, a shell's index with a recipe, or None.
    fn hit_type(&mut self, model: usize) -> Option<usize> {
        if model == 0 {
            return None;
        }
        if self.shells.is_empty() || self.shells_db != Self::db(2) {
            self.read_shells();
        }
        if model == Self::db(0) {
            return Some(0);
        }
        (2..8).find(|&k| self.shells[k].on && self.shells[k].model == model)
    }

    fn ours(&mut self, hit: usize) -> bool {
        let model = field(hit, HIT_MODEL);
        if self.hit_type(model).is_some() {
            return true;
        }
        self.s.down && self.down_ok && model != 0 && model == Self::db(1)
    }

    /// `ShieldEffect::Render` in the engine's place.
    pub fn feature(&mut self, v: &View) -> bool {
        let mut hits = Vec::new();
        walk(|_, sh| hits.push(sh));
        let mat = unsafe { rd::<usize>(SHIELD_MATERIAL) };
        for sh in hits {
            if sh == 0 || self.ours(sh) {
                continue;
            }
            unsafe {
                let f: unsafe extern "thiscall" fn(usize, usize, usize) = core::mem::transmute(FN_DRAWSHIELD);
                f(sh, v.cam, mat);
            }
        }
        true
    }

    /// The engine's live hits into ours: new ones born now, gone ones dying now.
    fn gather(&mut self, now: f64) {
        let model1 = Self::db(1);
        let s_fade = self.s.fade_out;
        let mut keep = (s_fade.max(self.s.ring_time)) as f64;
        keep = keep.max(self.s.glow_time as f64 * 3.0).max(self.s.duration as f64 * 3.0);
        for h in &mut self.hits {
            h.seen = false;
        }
        for d in &mut self.downs {
            d.seen = false;
        }
        let mut list = Vec::new();
        walk(|id, sh| list.push((id, sh)));
        for (id, sh) in list {
            if sh == 0 {
                continue;
            }
            let handle = field(sh, HIT_HANDLE) as i32;
            let o = object_of(handle);
            if o == 0 {
                continue;
            }
            let model = field(sh, HIT_MODEL);
            if model1 != 0 && model == model1 {
                let idx = self.downs.iter().position(|d| d.handle == handle);
                let idx = match idx {
                    Some(i) => Some(i),
                    None if self.downs.len() < 64 => {
                        self.downs.push(Down { handle, seen: false, birth: now, death: -1.0, st: [Strike::default(); 6] });
                        if self.logged_downs < 6 {
                            self.logged_downs += 1;
                            log!("effects: shields down on {} (hit {}, {})", obj::odf_name(o), id, if field_u8(sh, HIT_TIMED) != 0 { "timed" } else { "untimed" });
                        }
                        Some(self.downs.len() - 1)
                    }
                    None => None,
                };
                if let Some(i) = idx {
                    let d = &mut self.downs[i];
                    if d.death >= 0.0 {
                        d.birth = now - 0.08;
                        d.death = -1.0;
                    }
                    d.seen = true;
                }
            }
            let Some(ty) = self.hit_type(model) else { continue };
            let (dir, whole) = match hit_dir(sh, o) {
                Some(d) => (d, false),
                None if ty > 0 => (v3(0.0, 0.0, 1.0), true),
                None => continue,
            };
            let idx = match self.hits.iter().position(|h| h.id == id) {
                Some(i) => Some(i),
                None if self.hits.len() < 256 => {
                    let m = made_of(id);
                    let dmg = if ty == 0 { m.filter(|m| m.damage >= 0.0).map_or(self.s.damage_ref, |m| m.damage) } else { self.s.damage_ref };
                    let strength = libm::sqrtf(dmg / self.s.damage_ref).clamp(self.s.strength_min, self.s.strength_max);
                    let site = if ty != 0 { 0 } else { m.map_or(1, |m| m.site) };
                    self.hits.push(Hit { id, handle, site, ty, whole, dir, strength, birth: now, death: -1.0, seen: false });
                    if self.logged_hits < 12 {
                        self.logged_hits += 1;
                        log!(
                            "effects: shield hit {} on {} by a {}, damage {:.1} -> strength {:.2}; at {:.2} {:.2} {:.2}; stock {:.2} s{}, shields {:.2}",
                            id, obj::odf_name(o), SITE_NAME[site & 3], dmg, strength, dir.x, dir.y, dir.z,
                            engine::field_f32(sh, HIT_LEFT), if field_u8(sh, HIT_TIMED) != 0 { " left" } else { ", untimed" },
                            engine::field_f32(sh, HIT_SHIELDS)
                        );
                    }
                    Some(self.hits.len() - 1)
                }
                None => None,
            };
            if let Some(i) = idx {
                let h = &mut self.hits[i];
                h.handle = handle;
                h.dir = dir;
                h.seen = true;
                h.death = -1.0;
            }
        }
        let down_fade = self.s.down_fade as f64;
        self.downs.retain_mut(|d| {
            if !d.seen && d.death < 0.0 {
                d.death = now;
            }
            !((d.death >= 0.0 && now - d.death > down_fade) || object_of(d.handle) == 0)
        });
        self.hits.retain_mut(|h| {
            if !h.seen && h.death < 0.0 {
                h.death = now;
            }
            !((h.death >= 0.0 && now - h.death > keep) || now - h.birth > 600.0 || object_of(h.handle) == 0)
        });
    }

    /// Spot, ring radius, ring, glow of a hit now.
    fn hit_now(&self, h: &Hit, now: f64) -> (f32, f32, f32, f32) {
        let s = &self.s;
        let age = if now > h.birth { (now - h.birth) as f32 } else { 0.0 };
        let beam = h.site == 0;
        let alive = h.death < 0.0;
        let end = if alive { now } else { h.death };
        let out = if alive { 1.0 } else { (1.0 - (now - h.death) as f32 / s.fade_out).clamp(0.0, 1.0) };
        let flash = 1.0 + s.flash * libm::expf(-age / s.flash_time);
        let (spot, glow);
        if beam && alive {
            spot = h.strength * flash * (s.sustain + (1.0 - s.sustain) * libm::expf(-age / s.duration));
            glow = h.strength * (s.glow_sustain + (1.0 - s.glow_sustain) * libm::expf(-age / s.glow_time));
        } else if beam {
            let a = (h.death - h.birth) as f32;
            spot = h.strength * (s.sustain + (1.0 - s.sustain) * libm::expf(-a / s.duration)) * out;
            glow = h.strength * (s.glow_sustain + (1.0 - s.glow_sustain) * libm::expf(-a / s.glow_time)) * out;
        } else {
            spot = h.strength * flash * libm::expf(-age / s.duration);
            glow = h.strength * libm::expf(-age / s.glow_time);
        }
        let mut te = h.birth;
        if beam {
            let k = ((end - h.birth) / s.ring_period as f64) as i64;
            te = h.birth + k as f64 * s.ring_period as f64;
        }
        let tr = (now - te) as f32;
        let ring_r = s.ring_speed * tr;
        let ring = if tr < s.ring_time {
            let f = 1.0 - tr / s.ring_time;
            h.strength * f * f * if beam && te > h.birth { 0.6 } else { 1.0 }
        } else {
            0.0
        };
        (spot, ring_r, ring, glow)
    }

    fn strike_new(&self, d: &mut Down, k: usize, from: f64) {
        let s = &self.s;
        let st = &mut d.st[k];
        st.generation = st.generation.wrapping_add(1);
        let seed = (d.handle as u32).wrapping_mul(2654435761) ^ (k as u32).wrapping_mul(40503) ^ st.generation.wrapping_mul(2246822519);
        let mut y = rnd(seed) * 2.0 - 1.0;
        let a = rnd(seed.wrapping_add(1)) * 6.2831853;
        y = y * y * y;
        let r = libm::sqrtf(1.0 - y * y);
        st.dir = v3(r * libm::cosf(a), y, r * libm::sinf(a)).norm();
        st.t0 = from + (rnd(seed.wrapping_add(2)) * 2.0 * s.down_gap) as f64;
        st.dur = (s.down_time_min + (s.down_time_max - s.down_time_min) * rnd(seed.wrapping_add(3))) as f64;
        st.peak = 0.6 + 0.6 * rnd(seed.wrapping_add(4));
    }

    fn strike_now(&self, d: &mut Down, k: usize, now: f64) -> f32 {
        if d.st[k].generation == 0 || now - (d.st[k].t0 + d.st[k].dur) > 2.0 {
            self.strike_new(d, k, now);
        }
        let mut guard = 0;
        while now >= d.st[k].t0 + d.st[k].dur && guard < 1000 {
            let from = d.st[k].t0 + d.st[k].dur;
            self.strike_new(d, k, from);
            guard += 1;
        }
        let st = &d.st[k];
        let age = now - st.t0;
        if age < 0.0 {
            return 0.0;
        }
        let x = (age / st.dur) as f32;
        let a = (age as f32 / 0.03).clamp(0.0, 1.0);
        let e = (1.0 - x) * (1.0 - x);
        let tick = (now * 30.0) as i64 as u32;
        st.peak * a * e * (0.55 + 0.45 * rnd(tick.wrapping_mul(747796405).wrapping_add((k as u32).wrapping_mul(2891336453)).wrapping_add(d.handle as u32)))
    }

    fn down_level(&self, d: &Down, now: f64) -> f32 {
        if d.death >= 0.0 {
            (1.0 - (now - d.death) as f32 / self.s.down_fade).clamp(0.0, 1.0)
        } else {
            ((now - d.birth) as f32 / 0.08).clamp(0.0, 1.0)
        }
    }

    /// The shields-down light: per ship, its brightest discharge.
    pub fn lights(&mut self, v: &View) {
        if !self.on || !self.s.down || !self.down_ok || self.s.down_light <= 0.0 {
            return;
        }
        let now = v.time as f64;
        let mut n = 0;
        let mut downs = core::mem::take(&mut self.downs);
        for d in downs.iter_mut() {
            if n >= 8 {
                break;
            }
            let o = object_of(d.handle);
            if o == 0 || !obj::can_user_see(o) {
                continue;
            }
            let Some((a, t, r)) = ship_xform(o) else { continue };
            let mut best = 0.0;
            let mut kb = None;
            for k in 0..self.s.down_strikes {
                let e = self.strike_now(d, k, now);
                if e > best {
                    best = e;
                    kb = Some(k);
                }
            }
            let lv = self.down_level(d, now);
            if d.death < 0.0 {
                best += self.s.down_flash * libm::expf(-(now - d.birth) as f32 / self.s.down_flash_time);
            }
            let Some(kb) = kb else { continue };
            if best * lv < 0.01 {
                continue;
            }
            let mut br = engine::field_f32(field(o, 4), 0x40);
            if !(br > 0.01) {
                br = r;
            }
            let u = d.st[kb].dir;
            let pos = a[0] * u.x + a[1] * u.y + a[2] * u.z + t;
            let reach = self.s.down_light_reach * br;
            if self.lights.register(n, pos, self.s.down_light_col * (self.s.down_light * best * lv), reach, reach * 0.2) {
                n += 1;
            }
        }
        self.downs = downs;
    }

    pub fn draw(&mut self, v: &View, c: &Cam) {
        let now = v.time as f64;
        self.gather(now);
        if self.hits.is_empty() && !(self.down_ok && !self.downs.is_empty()) {
            return;
        }
        let dev = v.dev;
        let _g = gfx::Guard::new(dev);
        if !self.vs.bind(dev) || self.ps.get(dev).is_null() {
            return;
        }
        self.down_ok = !self.dps.get(dev).is_null();
        for (st, val) in [
            (RS_ZENABLE, 1), (RS_ZWRITEENABLE, 0), (RS_ZFUNC, CMP_LESSEQUAL), (RS_ALPHATESTENABLE, 0),
            (RS_ALPHABLENDENABLE, 1), (RS_SRCBLEND, BLEND_ONE), (RS_DESTBLEND, BLEND_ONE), (RS_BLENDOP, BLENDOP_ADD),
            (RS_SEPARATEALPHABLENDENABLE, 0), (RS_FOGENABLE, 0), (RS_CULLMODE, CULL_NONE), (RS_FILLMODE, 3),
            (RS_STENCILENABLE, 0), (152, 0), (RS_COLORWRITEENABLE, 7), (RS_SCISSORTESTENABLE, 0), (RS_SRGBWRITEENABLE, 0),
        ] {
            dev.set_rs(st, val);
        }
        dev.set_fvf(0x2);
        dev.vs_const(0, &c.vp);
        // The ordinary hits and the shells: per ship and type, its brightest eight.
        let mut done = alloc::vec![false; self.hits.len()];
        let mut drawn = 0;
        self.ps.bind(dev);
        for i in 0..self.hits.len() {
            if done[i] {
                continue;
            }
            let hi = self.hits[i];
            let mut pick: Vec<(usize, f32)> = Vec::new();
            for j in i..self.hits.len() {
                let hj = &self.hits[j];
                if done[j] || hj.handle != hi.handle || hj.ty != hi.ty || hj.whole != hi.whole {
                    continue;
                }
                done[j] = true;
                let (sp, _, rg, gl) = self.hit_now(hj, now);
                let w = sp + rg + gl;
                if w < 0.003 {
                    continue;
                }
                if pick.len() < 8 {
                    pick.push((j, w));
                } else if let Some(p) = pick.iter_mut().find(|p| p.1 < w) {
                    *p = (j, w);
                }
            }
            let o = object_of(hi.handle);
            if o == 0 || pick.is_empty() || !obj::can_user_see(o) {
                continue;
            }
            self.draw_ship(v, c, o, &pick, now, hi.ty, hi.whole);
            drawn += 1;
        }
        if self.s.down && self.down_ok && !self.downs.is_empty() {
            self.dps.bind(dev);
            let mut downs = core::mem::take(&mut self.downs);
            for d in downs.iter_mut() {
                let o = object_of(d.handle);
                if o == 0 || !obj::can_user_see(o) {
                    continue;
                }
                self.draw_down(v, c, o, d, now);
            }
            self.downs = downs;
        }
        if !self.said_first && drawn > 0 {
            self.said_first = true;
            log!("effects: first shield hits drawn: {} hit(s), sphere of {} triangles", self.hits.len(), self.sphere.len() / 3);
        }
    }

    fn sphere_draw(&self, dev: Device) {
        dev.draw_primitive_up(PT_TRIANGLELIST, (self.sphere.len() / 3) as u32, self.sphere.as_ptr() as *const u8, 12);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_ship(&self, v: &View, c: &Cam, o: usize, pick: &[(usize, f32)], now: f64, ty: usize, whole: bool) {
        let s = &self.s;
        let Some((a, t, r)) = ship_xform(o) else { return };
        ship_vsconst(v.dev, &a, t);
        let mut lvl = 1.0f32;
        if field(o, 0x14) & 8 != 0 {
            let (cur, max) = (engine::field_f32(o, 0x1c8), engine::field_f32(o, 0x1cc));
            lvl = if max > 0.05 { (cur / max).clamp(0.0, 1.0) } else { 0.0 };
        }
        let mut col = match s.colour_mode {
            ColMode::Stock => v3(1.0 - lvl, lvl, 0.0),
            ColMode::Fixed => s.fixed,
            ColMode::Faction => s.fac[fac_of(o)],
        };
        if s.colour_mode != ColMode::Stock {
            col = col + (s.low - col) * (s.low_shift * (1.0 - lvl));
        }
        let mut bright = s.brightness * (s.weak_level + (1.0 - s.weak_level) * lvl);
        if ty > 0 {
            col = self.shells[ty].col;
            bright = s.brightness * self.shells[ty].bright;
            lvl = 1.0;
        }
        if lvl < s.flicker_below {
            let weak = (s.flicker_below - lvl) / s.flicker_below;
            let tick = (now * s.flicker_rate as f64) as i64 as u32;
            let r1 = (super::hash32(tick.wrapping_mul(2654435761).wrapping_add(field(o, 0x28) as u32)) & 0xffff) as f32 / 65535.0;
            bright *= 1.0 - s.flicker * weak * r1;
        }
        let mut pc = [[0.0f32; 4]; 24];
        pc[0] = [c.eye.x, c.eye.y, c.eye.z, 1.0 / r];
        pc[1] = [col.x, col.y, col.z, bright];
        pc[2] = [1.0 / s.spot_size, 1.0 / s.ring_width, 1.0 / s.glow_reach, s.rim_power];
        pc[3] = [s.spot, s.ring, s.lattice, s.rim];
        if ty > 0 {
            let sh = &self.shells[ty];
            pc[3][0] = sh.spot;
            pc[3][1] = sh.ring;
            pc[3][2] = sh.lattice;
        }
        if whole {
            pc[2][2] = 0.3;
            pc[3][0] = 0.0;
        }
        let w = now - libm::floor(now / 1000.0) * 1000.0;
        pc[4] = [s.hex_cells, s.hex_line, w as f32, (s.crackle + (1.0 - lvl) * 0.5).clamp(0.0, 1.0)];
        pc[5] = [s.core, s.pattern, s.ring_lattice, 0.0];
        for (k, &(hi, _)) in pick.iter().enumerate().take(8) {
            let h = &self.hits[hi];
            let (spot, ring_r, ring, glow) = self.hit_now(h, now);
            let p = a[0] * h.dir.x + a[1] * h.dir.y + a[2] * h.dir.z + t;
            pc[8 + k] = [p.x, p.y, p.z, spot];
            pc[16 + k] = [ring_r, ring, glow, 0.0];
        }
        v.dev.ps_const(0, pc.as_flattened());
        self.sphere_draw(v.dev);
    }

    fn draw_down(&self, v: &View, c: &Cam, o: usize, d: &mut Down, now: f64) {
        let s = &self.s;
        let Some((a, t, r)) = ship_xform(o) else { return };
        ship_vsconst(v.dev, &a, t);
        let mut pc = [[0.0f32; 4]; 22];
        pc[0] = [c.eye.x, c.eye.y, c.eye.z, 1.0 / r];
        pc[1] = [s.down_arc.x, s.down_arc.y, s.down_arc.z, s.down_bright];
        pc[2] = [s.down_glow.x, s.down_glow.y, s.down_glow.z, s.down_haze];
        pc[3] = [s.down_scale, s.down_warp, s.down_core, s.down_glow_width];
        let ta = now * s.down_rate as f64;
        let tb = ta + 0.5;
        let sa = (ta as i64 as u32).wrapping_mul(2);
        let sb = (tb as i64 as u32).wrapping_mul(2).wrapping_add(1);
        let mut w = (ta - libm::floor(ta)) as f32;
        w = 1.0 - (2.0 * w - 1.0).abs();
        for j in 0..3u32 {
            pc[4][j as usize] = 5.0 + 50.0 * rnd(sa.wrapping_mul(2654435761).wrapping_add((d.handle as u32).wrapping_mul(97)).wrapping_add(j));
            pc[7][j as usize] = 5.0 + 50.0 * rnd(sb.wrapping_mul(2654435761).wrapping_add((d.handle as u32).wrapping_mul(97)).wrapping_add(j));
        }
        pc[4][3] = s.down_branches;
        let flash = if d.death < 0.0 { s.down_flash * libm::expf(-(now - d.birth) as f32 / s.down_flash_time) } else { 0.0 };
        pc[5] = [s.down_band, s.down_crackle, flash, self.down_level(d, now)];
        pc[6] = [s.down_far, s.down_rim_power, w, s.down_lit];
        for k in 0..s.down_strikes {
            let e = self.strike_now(d, k, now);
            let u = d.st[k].dir;
            let p = a[0] * u.x + a[1] * u.y + a[2] * u.z + t;
            pc[8 + k] = [p.x, p.y, p.z, e];
            pc[16 + k][0] = 1.0 / s.down_size;
        }
        v.dev.ps_const(0, pc.as_flattened());
        self.sphere_draw(v.dev);
    }
}

impl Default for Shields {
    fn default() -> Shields {
        Shields::new()
    }
}

/// Shells on by default, as the plugin's shipped Shields.ini had them.
fn default_shell_on(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "wcorbrfr" || n == "wshldadd"
}

fn default_shell(name: &str, s: &Set) -> ([f32; 3], f32, f32, f32, f32) {
    match name.to_ascii_lowercase().as_str() {
        "wcorbrfr" => ([255.0, 215.0, 120.0], 1.8, 0.8, 1.3, 1.3),
        "wshldadd" => ([120.0, 180.0, 255.0], 1.0, s.lattice, 0.8, s.ring),
        _ => ([255.0, 255.0, 255.0], 1.0, s.lattice, s.spot, s.ring),
    }
}

/// Walks `m_shieldList` in order (as `ShieldEffect::Render` does): (id, ShieldHit).
fn walk(mut f: impl FnMut(i32, usize)) {
    let head = unsafe { rd::<usize>(SHIELD_LIST + 4) };
    let nil = unsafe { rd::<usize>(SHIELD_NIL) };
    if head == 0 || nil == 0 {
        return;
    }
    let mut node = field(head, 0); // the leftmost: begin()
    let mut guard = 0;
    while node != 0 && node != head && guard < 4096 {
        guard += 1;
        f(field(node, 0xc) as i32, field(node, 0x14));
        // the in-order successor
        let mut next = field(node, 8);
        if next != nil {
            while field(next, 0) != nil {
                next = field(next, 0);
            }
            node = next;
        } else {
            let mut p = field(node, 4);
            while node == field(p, 8) {
                node = p;
                p = field(p, 4);
            }
            if field(node, 8) != p {
                node = p;
            }
        }
    }
}

fn inv3(a: &[f32; 9]) -> Option<[f32; 9]> {
    let det = a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6]) + a[2] * (a[3] * a[7] - a[4] * a[6]);
    if det.abs() < 1e-12 {
        return None;
    }
    let d = 1.0 / det;
    Some([
        (a[4] * a[8] - a[5] * a[7]) * d,
        (a[2] * a[7] - a[1] * a[8]) * d,
        (a[1] * a[5] - a[2] * a[4]) * d,
        (a[5] * a[6] - a[3] * a[8]) * d,
        (a[0] * a[8] - a[2] * a[6]) * d,
        (a[2] * a[3] - a[0] * a[5]) * d,
        (a[3] * a[7] - a[4] * a[6]) * d,
        (a[1] * a[6] - a[0] * a[7]) * d,
        (a[0] * a[4] - a[1] * a[3]) * d,
    ])
}

/// The hit's direction in shield space from its shell's transform (H S', H the hit's
/// frame, S' the shield-to-object 3x3 with its diagonal over the model's radius).
fn hit_dir(sh: usize, o: usize) -> Option<V3> {
    let s3: [f32; 9] = unsafe { rd(field(o, 0x40) + 0x38) };
    let x: [f32; 9] = unsafe { rd(sh + HIT_XFORM) };
    let r = engine::field_f32(sh, HIT_MODEL_RADIUS);
    if !(r > 1e-6) {
        return None;
    }
    let mut a = s3;
    a[0] /= r;
    a[4] /= r;
    a[8] /= r;
    let b = inv3(&a)?;
    let sc = engine::field_f32(sh, HIT_SCALE + 8);
    let vv = [x[6] * sc, x[7] * sc, x[8] * sc];
    let d = v3(
        vv[0] * b[0] + vv[1] * b[3] + vv[2] * b[6],
        vv[0] * b[1] + vv[1] * b[4] + vv[2] * b[7],
        vv[0] * b[2] + vv[1] * b[5] + vv[2] * b[8],
    );
    let l = d.len();
    if !(l > 1e-6) {
        return None;
    }
    Some(d * (1.0 / l))
}

/// A ship's shield ellipsoid in the world: w = u A + t for u on the unit sphere (A's
/// rows returned), and its mean radius (the cube root of its volume factor).
pub fn ship_xform(o: usize) -> Option<([V3; 3], V3, f32)> {
    let inst = field(o, 4);
    let s3: [f32; 12] = unsafe { rd(field(o, 0x40) + 0x38) };
    let tm: [f32; 12] = unsafe { rd(inst + 0x44) };
    let mut sc = [1.0f32; 3];
    if field_u8(inst, 0x74) != 0 {
        sc = unsafe { rd(inst + 0x28) };
    }
    let trow = |i: usize| v3(tm[i * 3], tm[i * 3 + 1], tm[i * 3 + 2]);
    let tr = [trow(0), trow(1), trow(2)];
    let tp = v3(tm[9], tm[10], tm[11]);
    let mut a = [V3::ZERO; 3];
    for (i, ai) in a.iter_mut().enumerate() {
        let m = [s3[i * 3] * sc[0], s3[i * 3 + 1] * sc[1], s3[i * 3 + 2] * sc[2]];
        *ai = tr[0] * m[0] + tr[1] * m[1] + tr[2] * m[2];
    }
    let t = tr[0] * (s3[9] * sc[0]) + tr[1] * (s3[10] * sc[1]) + tr[2] * (s3[11] * sc[2]) + tp;
    let det = a[0].dot(a[1].cross(a[2]));
    if det.abs() < 1e-9 {
        return None;
    }
    let r = libm::powf(det.abs(), 1.0 / 3.0);
    if r > 0.01 { Some((a, t, r)) } else { None }
}

/// The vertex shader's constants for one ellipsoid (c4..c9): the columns of the unit
/// sphere -> world map, and the rows of its normal matrix.
fn ship_vsconst(dev: Device, a: &[V3; 3], t: V3) {
    let m = [a[0].x, a[0].y, a[0].z, a[1].x, a[1].y, a[1].z, a[2].x, a[2].y, a[2].z];
    let b = inv3(&m).unwrap_or([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
    let tt = [t.x, t.y, t.z];
    let mut vc = [[0.0f32; 4]; 6];
    for j in 0..3 {
        vc[j] = [m[j], m[3 + j], m[6 + j], tt[j]];
    }
    for k in 0..3 {
        vc[3 + k] = [b[k * 3], b[k * 3 + 1], b[k * 3 + 2], 0.0];
    }
    dev.vs_const(4, vc.as_flattened());
}

fn fac_of(o: usize) -> usize {
    let n = obj::odf_name(o);
    let Some(c) = n.bytes().next() else { return 6 };
    match c.to_ascii_lowercase() {
        b'8' => 5,
        b'f' => 0,
        b'k' => 1,
        b'r' => 2,
        b'b' => 3,
        b'c' => 4,
        _ => 6,
    }
}

/// The unit sphere, a triangle list: 18 rings by 36 meridians.
fn make_sphere() -> Vec<[f32; 3]> {
    const RINGS: usize = 18;
    const SEGS: usize = 36;
    let mut out = Vec::new();
    let pi = core::f32::consts::PI;
    for r in 0..RINGS {
        let (t0, t1) = (pi * r as f32 / RINGS as f32, pi * (r + 1) as f32 / RINGS as f32);
        for s in 0..SEGS {
            let (p0, p1) = (2.0 * pi * s as f32 / SEGS as f32, 2.0 * pi * (s + 1) as f32 / SEGS as f32);
            let pt = |t: f32, p: f32| [libm::sinf(t) * libm::cosf(p), libm::cosf(t), libm::sinf(t) * libm::sinf(p)];
            let q = [pt(t0, p0), pt(t0, p1), pt(t1, p1), pt(t1, p0)];
            if r > 0 {
                out.extend_from_slice(&[q[0], q[1], q[2]]);
            }
            if r < RINGS - 1 {
                out.extend_from_slice(&[q[0], q[2], q[3]]);
            }
        }
    }
    out
}
