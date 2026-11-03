//! The scene lights: `[Lights]` of Render3D.ini (`Replace`, `KeyColour`, `KeyAxis`,
//! `FillColour`, `FillAxis`, `SkyLight`), `Lighting.asi`'s `Lights=1` and its settled
//! values. They are the hull look's lights, so the hulls module owns the keys and the
//! `Lighting.asi` test; the core applies them to the list it gathers
//! (`scene::lights::Lights::replace_scene_lights`) through `services`, whether the
//! module draws hulls or not (a module at 0 leaves the other modules' lights as they
//! were: the map's directional lights stay replaced).
use r3d_core::math::v3;
use crate::rt::Ini;
use r3d_core::scene::lights::SceneConfig;

/// The keys as last read: `replace` is the ini's own; `Lighting.asi` is asked at the
/// first gather (plugins are all loaded by the first frame, not at load).
static mut KEYS: Option<SceneConfig> = None;
/// The answer for `Lighting.asi`: 0 not asked, 1 loaded, 2 not.
static mut BESIDE: u8 = 0;
/// What was last logged.
static mut SAID: Option<SceneConfig> = None;

/// Reads `[Lights]`' scene keys (at load and at every settings change).
#[allow(static_mut_refs)]
pub fn configure(ini: &Ini) {
    let v = |k: &str, d: [f32; 3]| {
        let a = ini.floats("Lights", k, d);
        v3(a[0], a[1], a[2])
    };
    let key_axis = v("KeyAxis", [0.50, -0.50, 0.71]).norm();
    let c = SceneConfig {
        replace: ini.int("Lights", "Replace", 1) != 0,
        key: v("KeyColour", [1.00, 0.96, 0.90]),
        key_axis,
        fill: v("FillColour", [0.06, 0.08, 0.18]),
        fill_axis: v("FillAxis", [-key_axis.x, -key_axis.y, -key_axis.z]).norm(),
        sky: ini.float("Lights", "SkyLight", 0.35),
    };
    unsafe { KEYS = Some(c) };
}

/// The provider the core asks for the scene lights.
pub struct Provider;

impl r3d_core::services::SceneLightSource for Provider {
    #[allow(static_mut_refs)]
    fn scene_config(&self) -> Option<SceneConfig> {
        let mut c = unsafe { KEYS }?;
        unsafe {
            if BESIDE == 0 {
                BESIDE = if r3d_core::plugin_loaded("Lighting.asi") { 1 } else { 2 };
            }
            // Nothing is changed while `Lighting.asi` is loaded: it has replaced them in
            // the engine's list already.
            c.replace = c.replace && BESIDE == 2;
            if SAID != Some(c) {
                SAID = Some(c);
                crate::log!(
                    "lights: scene lights {} (key {:?} along {:?})",
                    if c.replace { "replace the map's" } else { "are the map's" }, c.key, c.key_axis
                );
            }
        }
        Some(c)
    }
}

pub static PROVIDER: Provider = Provider;
