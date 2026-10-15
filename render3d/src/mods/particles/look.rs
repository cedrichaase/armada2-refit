//! A `@sprite_node` resolved for drawing: its sprite's texture and rectangle, its
//! material, size, colour and animations (`spec-particles.md`, 4.2 and 8). Shared by
//! particles (whose emitter names one) and SOD sprite nodes (nebula puffs).
use super::anim::{Track, Val};
use crate::assets;
use crate::math::{v3, V3};
use alloc::collections::BTreeMap;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

/// How a sprite is blended (its `@tmaterial`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Material {
    /// `additive`: ONE, ONE, mipmapped.
    Additive,
    /// `add_nomipf`: additive without mip filtering.
    AdditiveNoMip,
    /// `translucent`: SRCALPHA, INVSRCALPHA.
    Translucent,
    /// `alphathreshold` (and anything else): opaque, alpha-tested.
    Cutout,
}

pub struct Look {
    pub id: String,
    /// Texture name; None when the sprite is missing (the node draws nothing).
    pub texture: Option<String>,
    /// u, v, w, h in 0..1.
    pub uv: [f32; 4],
    pub material: Material,
    /// Billboard: half extents; oriented: full extents.
    pub size: [f32; 2],
    pub colour: V3,
    pub billboard: bool,
    /// The node's own animation (colour or draw).
    pub anim: Option<Track>,
    /// The sprite's `@anim` tracks (texture offsets).
    pub sprite_anims: Vec<Track>,
}

impl Look {
    /// Colour and draw flag at time `t` on the node's animation clock.
    pub fn state(&self, t: f32, flags: u32) -> (V3, bool) {
        match self.anim.as_ref().map(|a| a.eval(t, flags)) {
            Some(Val::Colour(c)) => (c, true),
            Some(Val::Draw(d)) => (self.colour, d),
            _ => (self.colour, true),
        }
    }
    /// The texture rectangle at `t` on the sprite's clock (its flipbook offset added).
    pub fn uv_at(&self, t: f32, flags: u32) -> [f32; 4] {
        let o = super::anim::offset(&self.sprite_anims, t, flags);
        [self.uv[0] + o[0], self.uv[1] + o[1], self.uv[2], self.uv[3]]
    }
}

static mut LOOKS: BTreeMap<String, Option<Rc<Look>>> = BTreeMap::new();

/// The look of a `@sprite_node` by id (case-insensitive), made once.
#[allow(static_mut_refs)]
pub fn get(id: &str) -> Option<Rc<Look>> {
    let key = crate::rt::lower(id);
    let cache = unsafe { &mut LOOKS };
    if let Some(l) = cache.get(&key) {
        return l.clone();
    }
    let l = build(&key).map(Rc::new);
    if l.is_none() {
        crate::log!("particles: no sprite node {}", key);
    }
    cache.insert(key, l.clone());
    l
}

fn build(id: &str) -> Option<Look> {
    let t = assets::sprites();
    let nd = t.sprite_node(id)?;
    let sprite = t.sprite(&nd.sprite);
    let (texture, uv, material, sprite_anims) = match sprite {
        Some(s) => {
            let rw = s.reference[0].filter(|r| *r > 0.0).unwrap_or(128.0);
            let rh = s.reference[1].filter(|r| *r > 0.0).unwrap_or(rw);
            let uv = [s.rect[0] / rw, s.rect[1] / rh, s.rect[2] / rw, s.rect[3] / rh];
            let m = match s.material.as_str() {
                "additive" => Material::Additive,
                "add_nomipf" => Material::AdditiveNoMip,
                "translucent" => Material::Translucent,
                _ => Material::Cutout,
            };
            let anims = s.anims.iter().filter_map(|a| t.animation(a)).filter_map(Track::new).collect();
            (Some(s.texture.clone()), uv, m, anims)
        }
        None => (None, [0.0; 4], Material::Additive, Vec::new()),
    };
    let anim = nd.animation.as_deref().and_then(|a| t.animation(a)).and_then(Track::new);
    Some(Look {
        id: String::from(id),
        texture,
        uv,
        material,
        size: nd.size,
        colour: v3(nd.colour[0], nd.colour[1], nd.colour[2]),
        billboard: nd.billboard,
        anim,
        sprite_anims,
    })
}
