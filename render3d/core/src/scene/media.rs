//! Light semantics as data (README.md, "Clustered lights", "Media"): what a light is to
//! the nebulae's gas (its *medium*) and the flags a light carries, packed into the word
//! the shaders read (`ClLight.t`, `shaders/clustered.hlsli`).
//!
//! Pure data and arithmetic, no engine and no allocation: `render3d/hosttest` compiles
//! this file on the host and asserts the packed word is the one the old `Gas` enum gave.
//!
//! A medium is a *response* of the gas's march (`world_nebula.hlsl`, `nl_light`), and
//! the march knows its responses by the literal it reads from `t.z`. So the slot a
//! medium takes is fixed by its response, in `MEDIA` (the table of exactly what the
//! shader hard-codes), not by the order modules register in: registering is naming a
//! slot, and the same parameters always give the same id, whoever registers first.
//! Slots 5..7 are reserved (the shader has no response for them yet).

/// `t.x`: the kind of light, in its low four bits.
pub const KIND_POINT: u32 = 0;
pub const KIND_LINE: u32 = 1;
pub const KIND_SPOT: u32 = 2;

/// `t.x` bit 4: the light lights what is near it times the share of a sphere about it
/// that faces the Key (`b.xyz`), and not the sphere's own surface (`Lights::share`).
/// A planet's day side is the one user (world sets it).
pub const FLAG_SHARE: u32 = 16;

/// The most media (the slots `t.z` can name).
pub const SLOTS: usize = 8;

/// A light's medium: the slot `t.z` carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, PartialOrd, Ord)]
pub struct MediumId(pub u8);

impl MediumId {
    /// Slot 0, the core's: any light no module said otherwise of.
    pub const PLAIN: MediumId = MediumId(0);
}

/// How the gas's march takes a light (`nl_light`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Response {
    /// Scattered as it falls off on the plating (`cl_eval`).
    Scatter,
    /// As `Scatter`, then a core about the breach falling as the inverse square over a
    /// wash (`LitExplosionCore=`, `LitExplosionFar=`).
    Fireball,
    /// A corona close about it on a soft tail out to its reach (`LitCorona=`), never a
    /// ball of gas lit its reach wide.
    Corona,
    /// It lights what is near it, never the gas.
    Unlit,
}

/// Which of the gas's shares (`k_nlk`, `[Nebulae] Lit*=`) scales the light.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Share {
    /// None: the light as it is (a line light takes `LitBeam=` by its kind, not here).
    Whole,
    /// `LitExplosion=` (`k_nlk.x`).
    Explosion,
    /// `LitBolt=` (`k_nlk.y`).
    Bolt,
    /// `LitSpark=` (`k_nlk.w`).
    Spark,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MediumParams {
    pub response: Response,
    pub share: Share,
}

/// The core's medium, slot 0.
pub const PLAIN: MediumParams = MediumParams { response: Response::Scatter, share: Share::Whole };
/// effects: a fireball's light.
pub const FIREBALL: MediumParams = MediumParams { response: Response::Fireball, share: Share::Explosion };
/// ordnance: a torpedo's or pulse's light that rides with the shot.
pub const BOLT: MediumParams = MediumParams { response: Response::Corona, share: Share::Bolt };
/// world: a nebula's own glow.
pub const OWN_GLOW: MediumParams = MediumParams { response: Response::Unlit, share: Share::Whole };
/// effects (and hulls' sensor glows): a small light at a hull, a weld, a shields-down
/// discharge.
pub const SPARK: MediumParams = MediumParams { response: Response::Corona, share: Share::Spark };

/// Each slot's response, as `world_nebula.hlsl` reads `t.z` (`gk == 1u` a fireball,
/// `gk == 2u || gk == 4u` a corona in `LitBolt=` or `LitSpark=`, `gk == 3u` skipped).
/// The slot numbers are the old `Gas` enum's, so the packed word is unchanged.
pub const MEDIA: [Option<MediumParams>; SLOTS] = [Some(PLAIN), Some(FIREBALL), Some(BOLT), Some(OWN_GLOW), Some(SPARK), None, None, None];

/// The slot of a medium with these parameters; None if the shader has no response for
/// them (a reserved slot).
pub fn slot_of(p: MediumParams) -> Option<MediumId> {
    MEDIA.iter().position(|m| *m == Some(p)).map(|i| MediumId(i as u8))
}

/// The word `t` of a point light: its kind, `FLAG_SHARE` if it takes a share, its
/// group (lights of one colour share one), its medium.
pub fn point_word(share: bool, group: u32, medium: MediumId) -> [u32; 4] {
    [KIND_POINT | if share { FLAG_SHARE } else { 0 }, group, medium.0 as u32, 0]
}

/// The word `t` of a shape of render3d's own (`KIND_LINE`, `KIND_SPOT`), its group set
/// when the view's lights are grouped.
pub fn shape_word(kind: u32) -> [u32; 4] {
    [kind, 0, MediumId::PLAIN.0 as u32, 0]
}
