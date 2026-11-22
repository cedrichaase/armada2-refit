//! The game object's instance, as every module sees it (README, "Module interface",
//! "Services"): the frustum test, the scale effects, the team colour, the element names
//! and the nodes an instance shows. Lifted out of the hulls module so effects, ordnance,
//! particles, world and overlays no longer reach into it. The engine reads are
//! `engine::instance`; this layer holds none of its own.
use crate::engine::instance as eng;
use crate::scene::{pose, Model};
use alloc::vec::Vec;

pub use eng::{element_name, frustum_test, instance_scale, render_fn, set_on_screen, team_colour};

/// The nodes an instance shows as the engine would draw it, without its object: for a
/// fog-of-war ghost (overlays), whose spirit keeps the instance as it was last seen.
pub fn instance_visibility(inst: usize, model: &Model, p: &mut pose::Pose) {
    visibility(0, inst, eng::is_craft_instance(inst), model, p);
}

/// The nodes the engine shows or hides per instance, by writing flags into the shared
/// database before it draws (`ST3D_Stream::ReadDatabase`, `GameObjectInstance::
/// RenderInternal`, `CraftInstance::RenderInternal`, `Shipyard::Render`,
/// `MiningStation::Render`), applied to this instance's pose instead.
pub fn visibility(o: usize, inst: usize, craft: bool, model: &Model, p: &mut pose::Pose) {
    let sod = &model.sod;
    let hide_tree = |p: &mut pose::Pose, i: usize| hide(sod, i, &mut p.shown, true);
    let hide_children = |p: &mut pose::Pose, i: usize| {
        for &c in &sod.nodes[i].children {
            hide(sod, c, &mut p.shown, true);
        }
    };
    // `hardpoints`: never drawn.
    if let Some(i) = sod.find("hardpoints") {
        hide_tree(p, i);
    }
    // `lights` (or `light`): hidden at detail level 0 and while the instance's `+0x90`
    // says so.
    if eng::lights_hidden(inst) || eng::detail_level() == 0 {
        if let Some(i) = sod.find("lights").or_else(|| sod.find("light")) {
            hide_tree(p, i);
        }
    }
    if craft {
        let class = eng::instance_class(inst);
        for (cf, ib) in eng::damage_nodes() {
            let n = eng::class_node(class, cf);
            if n == 0 {
                continue;
            }
            if let Some(i) = eng::element_name(n).and_then(|nm| sod.find(nm)) {
                if eng::instance_byte(inst, ib) != 0 {
                    hide_children(p, i);
                }
            }
        }
        // The crew fires: under `crew`, node after node, one more hidden for each
        // fifth of the crew (`CrewDamageTraverse`: hidden from `+0xa4` + 0.2 k >= 1).
        // The engine's child lists run in reverse file order (`ST3D_Node::AddChild`
        // puts each new child first), so `crew1`, stored last, is the first shown.
        let crew = eng::crew_node(class);
        if crew != 0 {
            if let Some(i) = eng::element_name(crew).and_then(|nm| sod.find(nm)) {
                let mut x = eng::crew_fire(inst);
                let mut stack: Vec<usize> = sod.nodes[i].children.iter().copied().collect();
                let mut guard = 0;
                while let Some(n) = stack.pop() {
                    guard += 1;
                    if guard > sod.nodes.len() {
                        break;
                    }
                    if x >= 1.0 {
                        p.shown[n] = false;
                    }
                    x += 0.2;
                    stack.extend(sod.nodes[n].children.iter().copied());
                }
            }
        }
        // `borg`: shown only while assimilated.
        let borg = eng::borg_node(class);
        if borg != 0 && !eng::assimilated(inst) {
            if let Some(i) = eng::element_name(borg).and_then(|nm| sod.find(nm)) {
                hide_tree(p, i);
            }
        }
    }
    if let Some((n, hidden)) = eng::station_node(o) {
        if hidden {
            if let Some(i) = eng::element_name(n).and_then(|nm| sod.find(nm)) {
                hide_tree(p, i);
            }
        }
    }
}

/// Marks a node (and, with `tree`, its subtree) as not shown.
pub fn hide(sod: &a2fmt::sod::Sod, root: usize, shown: &mut [bool], tree: bool) {
    let mut stack = alloc::vec![root];
    let mut guard = 0;
    while let Some(i) = stack.pop() {
        guard += 1;
        if guard > sod.nodes.len() {
            break;
        }
        shown[i] = false;
        if tree {
            stack.extend_from_slice(&sod.nodes[i].children);
        }
    }
}
