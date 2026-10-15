//! The hulls' look, `[Hulls]` in Render3D.ini: Lighting.asi's settled values
//! (lighting/README.md, "Settings"), carried over with their names.
use crate::config;
use crate::math::{v3, V3};

#[derive(Clone, Copy)]
pub struct Profile {
    pub ambient: V3,
    pub specular: f32,
    pub specular_power: f32,
    pub rim: V3,
    pub sun: f32,
    pub self_illum: f32,
}

#[derive(Clone)]
pub struct Look {
    pub hull: Profile,
    /// The bump-mapped hulls (in stock, the Borg alone): "The Borg profile".
    pub borg: Profile,
    pub rim_power: f32,
    /// Lighting.asi's highlight knee: lit colour rolls off towards white above it and
    /// stays within 1, so a hot spot (a phaser's emitter, a discharge) lights the
    /// plating without blooming over the hull (1: clip, above: no shoulder at all).
    pub knee: f32,
    pub bump: f32,
    pub point_lights: usize,
    pub near_fade: bool,
    pub near_fade_depth: f32,
    pub near_fade_min: f32,
    pub shadows: bool,
    pub shadow_size: u32,
    pub shadow_strength: f32,
    pub planet_shadows: bool,
    pub cloak: bool,
    pub cloak_bend: f32,
    pub cloak_split: f32,
    pub cloak_ripple: f32,
    pub cloak_speed: f32,
    pub cloak_tint: V3,
    pub cloak_rim: f32,
    pub cloak_rim_colour: V3,
    pub cloak_glint: f32,
    pub cloak_hull: f32,
    pub cloak_front: f32,
    pub cloak_front_colour: V3,
    /// Sprite nodes (running lights) and billboard effects: their colour times this
    /// (above 1 they bloom).
    pub sprites: f32,
    /// Additive meshes (engine glows and the like): their colour times this.
    pub additive: f32,
    /// Whether team groups take the team's colour.
    pub team: bool,
    /// Whether Scrap objects (dilithium moons, comets) are drawn here.
    pub scrap: bool,
    /// The bench's A/B switch (`ABTest=1`): see `Hulls::superseded`.
    pub ab_test: bool,
}

fn v(a: [f32; 3]) -> V3 {
    v3(a[0], a[1], a[2])
}

pub fn load() -> Look {
    let ini = config::ini();
    let s = "Hulls";
    let f = |k: &str, d: f32| ini.float(s, k, d);
    let c = |k: &str, d: [f32; 3]| v(ini.floats(s, k, d));
    let self_illum = f("SelfIllumination", 1.8);
    let hull = Profile {
        ambient: c("Ambient", [0.05, 0.05, 0.07]),
        specular: f("Specular", 0.7),
        specular_power: f("SpecularPower", 24.0),
        rim: c("RimLight", [0.12, 0.14, 0.20]),
        sun: f("HullSun", 1.1),
        self_illum,
    };
    let borg = Profile {
        ambient: c("BorgAmbient", [0.015, 0.018, 0.015]),
        specular: f("BorgSpecular", 0.15),
        specular_power: f("BorgSpecularPower", 64.0),
        rim: c("BorgRimLight", [0.06, 0.13, 0.03]),
        sun: f("BorgSun", 0.8),
        self_illum: f("BorgSelfIllumination", self_illum),
    };
    Look {
        hull,
        borg,
        rim_power: f("RimPower", 3.0),
        knee: f("HighlightKnee", 0.8),
        bump: f("BumpStrength", 1.0),
        point_lights: ini.int(s, "PointLights", 12).clamp(0, 16) as usize,
        near_fade: ini.int(s, "NearFade", 1) != 0,
        near_fade_depth: f("NearFadeDepth", 0.3).max(0.01),
        near_fade_min: f("NearFadeMin", 0.125),
        shadows: ini.int(s, "Shadows", 1) != 0,
        shadow_size: ini.int(s, "ShadowSize", 2048).clamp(512, 8192) as u32,
        shadow_strength: f("ShadowStrength", 1.0),
        planet_shadows: ini.int(s, "PlanetShadows", 1) != 0,
        cloak: ini.int(s, "Cloak", 1) != 0,
        cloak_bend: f("CloakBend", 0.035),
        cloak_split: f("CloakSplit", 0.18),
        cloak_ripple: f("CloakRipple", 0.08),
        cloak_speed: f("CloakSpeed", 1.0),
        cloak_tint: c("CloakTint", [0.93, 0.97, 1.0]),
        cloak_rim: f("CloakRim", 0.35),
        cloak_rim_colour: c("CloakRimColour", [0.45, 0.75, 1.0]),
        cloak_glint: f("CloakGlint", 0.5),
        cloak_hull: f("CloakHull", 0.0),
        cloak_front: f("CloakFront", 1.0),
        cloak_front_colour: c("CloakFrontColour", [0.3, 1.0, 0.55]),
        sprites: f("SpriteNodes", 1.5),
        additive: f("AdditiveMeshes", 1.0),
        team: ini.int(s, "TeamColours", 1) != 0,
        scrap: ini.int(s, "Scrap", 1) != 0,
        ab_test: ini.int(s, "ABTest", 0) != 0,
    }
}
