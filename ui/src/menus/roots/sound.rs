//! Roots' Sound Settings (`TSoundSettingsForm`) as Armada II's sound screen
//! (`screens::sound_settings`): the four volumes as sliders, unit speech and hardware
//! acceleration as check boxes, the sound device and the speaker set-up as dropdowns over
//! the form's combo boxes, and the number of sound channels as a slider with its count.
//! Roots' own handlers write the profile as a control changes; Previous Menu is Roots' own.
use super::kit::{self, Column};
use crate::engine::api::forms::Form;
use crate::menus::screens::form as sform;
use crate::widgets::Ui;

const VOLUMES: &[(&str, &str)] = &[
    ("lblMusicVolume", "sldrMusicVolume"),
    ("lblVoiceVolume", "sldrVoiceVolume"),
    ("lblEffectsVolume", "sldrEffectsVolume"),
    ("lblButtonVolume", "sldrButtonVolume"),
];

pub fn draw(ui: &mut Ui, f: &Form) -> bool {
    let (Some(device), Some(speakers), Some(channels)) =
        (kit::shown(f, "cbbSoundDevice"), kit::shown(f, "cbbSpeakerSetup"), kit::shown(f, "sldrSoundChannels"))
    else {
        return false;
    };
    if VOLUMES.iter().any(|(_, s)| kit::shown(f, s).is_none()) {
        return false;
    }
    let slider_h = kit::LABEL + kit::SLIDER + kit::GAP;
    let left = VOLUMES.len() as f32 * slider_h;
    let right = 2.0 * (kit::CHECK + kit::CHECK_GAP) + 2.0 * (kit::FIELD + kit::GAP) + slider_h;
    let (body, foot) = sform::frame(ui, 1180.0, left.max(right), "SOUND");
    let gap = ui.u(40.0);
    let cw = (body.w - gap) * 0.5;
    let top = body.y + ui.u(6.0);

    let mut l = Column::new(body.x, top, cw);
    for (label, slider) in VOLUMES {
        let c = kit::shown(f, slider).unwrap_or(device);
        let r = l.field(ui, kit::text(f, label), kit::SLIDER);
        kit::slider(ui, f, c, r);
    }

    let mut r = Column::new(body.x + cw + gap, top, cw);
    if let Some(c) = kit::shown(f, "chkUnitSpeech") {
        let row = r.row(ui, kit::CHECK, kit::CHECK_GAP);
        kit::check(ui, f, c, row);
    }
    for c in [device, speakers] {
        let row = r.field(ui, "", kit::FIELD);
        kit::combo(ui, f, c, row);
    }
    if let Some(c) = kit::shown(f, "chkHardware") {
        let row = r.row(ui, kit::CHECK, kit::CHECK_GAP);
        kit::check(ui, f, c, row);
    }
    let count = kit::text(f, "lblSoundChannelCount");
    let caption = crate::sfmt!("{}  {}", kit::text(f, "lblSoundChannels"), count);
    let row = r.field(ui, &caption, kit::SLIDER);
    kit::slider(ui, f, channels, row);
    kit::back(ui, f, foot, "buttonBack");
    true
}
