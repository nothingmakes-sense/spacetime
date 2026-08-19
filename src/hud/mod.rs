//! Screen-space inventory HUD.
//!
//! # Data flow
//! - **In:** [`ItemView`] (authoritative stacks), [`ItemUi`] (open/held/debug),
//!   GPU [`ItemMeshes`], optional [`DebugSnap`].
//! - **Out:** [`DrawRequest`]s consumed by `VulkanContext::draw_model` after
//!   `begin_overlay`, plus [`hit_slot`] for click-to-move.
//!
//! Overlay camera is identity + Y-flip, so these positions are NDC and never
//! follow the chase cam (that was the rotate/stutter bug).

use std::collections::HashMap;

use anyhow::Result;
use glam::{Mat4, Quat, Vec3};

use crate::assets::{
    digit_quad, glyph_quad, item_gem, load_rgba_png, resolve_asset, slot_plate, sprite_quad,
};
use crate::items::{
    selected_recipe, ItemId, ItemUi, ItemView, SlotRef, Stack, StationKind, BAG_SLOTS, CATALOG,
    HOTBAR, RESOURCE_BITS_DIR,
};
use crate::scene::DrawRequest;
use crate::vulkan::{ModelHandle, VulkanContext};

/// Rotate the XZ-flat slot plate so its face is toward the overlay camera (−Z).
const FACE_CAM: f32 = std::f32::consts::FRAC_PI_2;

const HOT_Y: f32 = -0.86;
const HOT_STEP: f32 = 0.125;
const HOT_SIZE: f32 = 0.105;
const BAG_Y: f32 = 0.30;
const BAG_STEP: f32 = 0.118;
const BAG_SIZE: f32 = 0.10;
const ST_X: f32 = 0.72;
const ST_Y: f32 = 0.30;

pub struct ItemMeshes {
    pub by_item: HashMap<u16, ModelHandle>,
    pub by_mesh: HashMap<String, ModelHandle>,
    pub slot: ModelHandle,
    pub slot_sel: ModelHandle,
    pub slot_station: ModelHandle,
    pub slot_panel: ModelHandle,
    pub slot_border: ModelHandle,
    pub digits: [ModelHandle; 10],
    pub glyphs: HashMap<char, ModelHandle>,
    pub chest_body: ModelHandle,
    pub chest_lid: ModelHandle,
    pub furnace: ModelHandle,
    pub ember: ModelHandle,
    pub workbench: ModelHandle,
}

impl ItemMeshes {
    /// Upload slot sprites, ResourceBits meshes, digits, and station props.
    pub fn upload(
        vk: &mut VulkanContext,
        chest_body: ModelHandle,
        chest_lid: ModelHandle,
        furnace: ModelHandle,
        ember: ModelHandle,
        workbench: ModelHandle,
    ) -> Result<Self> {
        let mut by_item = HashMap::new();
        let mut by_mesh = HashMap::new();
        for def in CATALOG {
            by_item.insert(def.id.0, vk.upload_model(&item_gem(def.color))?);
            for stem in std::iter::once(def.mesh).chain(def.tiers.iter().map(|(_, s)| *s)) {
                if stem.is_empty() || by_mesh.contains_key(stem) {
                    continue;
                }
                let path = resolve_asset(format!("{RESOURCE_BITS_DIR}/{stem}.gltf"));
                match crate::assets::load_gltf(&path) {
                    Ok(model) => match vk.upload_model(&model) {
                        Ok(h) => {
                            by_mesh.insert(stem.to_string(), h);
                        }
                        Err(e) => log::warn!("upload resource bit {stem}: {e:#}"),
                    },
                    Err(e) => log::warn!("resource bit {stem}: {e:#}"),
                }
            }
        }
        let mut digits = [ModelHandle(0); 10];
        for d in 0..10u8 {
            digits[d as usize] = vk.upload_model(&digit_quad(d))?;
        }
        let mut glyphs = HashMap::new();
        const GLYPHS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ .:-+/_";
        for ch in GLYPHS.chars() {
            glyphs.insert(ch, vk.upload_model(&glyph_quad(ch))?);
        }
        Ok(Self {
            by_item,
            by_mesh,
            slot: load_slot(vk, "wood")?,
            slot_sel: load_slot(vk, "orange_red")?,
            slot_station: load_slot(vk, "coldsteel")?,
            slot_panel: vk.upload_model(&slot_plate([1.0, 1.0, 1.0, 0.22], "panel"))?,
            slot_border: vk.upload_model(&slot_plate([0.05, 0.05, 0.05, 0.55], "border"))?,
            digits,
            glyphs,
            chest_body,
            chest_lid,
            furnace,
            ember,
            workbench,
        })
    }

    pub fn item(&self, id: ItemId) -> Option<ModelHandle> {
        self.visual(id, 1)
    }

    /// Mesh for this stack size — upgrades when nearby drops auto-merge.
    pub fn visual(&self, id: ItemId, count: u16) -> Option<ModelHandle> {
        let stem = id.visual_mesh(count);
        if !stem.is_empty() {
            if let Some(h) = self.by_mesh.get(stem) {
                return Some(*h);
            }
        }
        self.by_item.get(&id.0).copied()
    }

    pub fn station_body(&self, kind: StationKind) -> ModelHandle {
        match kind {
            StationKind::Chest => self.chest_body,
            StationKind::Furnace => self.furnace,
            StationKind::Workbench => self.workbench,
        }
    }
}

fn load_slot(vk: &mut VulkanContext, name: &str) -> Result<ModelHandle> {
    let (w, h, px) = load_rgba_png(format!("assets/ui/slots/{name}.png"))?;
    vk.upload_model(&sprite_quad(px, w, h, name))
}

/// Snapshot of sim state for the F3 overlay. Filled in `App::render`.
pub struct DebugSnap {
    pub fps: f32,
    pub pos: Vec3,
    pub vel: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub grounded: bool,
    pub sitting: bool,
    pub bag_open: bool,
    pub selected: usize,
    pub held: Stack,
    pub station: Option<&'static str>,
    pub loot: usize,
    pub multiplayer: bool,
}

/// Axis-aligned HUD rectangle in NDC (center + half-extents).
#[derive(Clone, Copy)]
pub struct HudRect {
    pub x: f32,
    pub y: f32,
    pub hw: f32,
    pub hh: f32,
}

impl HudRect {
    fn contains(self, px: f32, py: f32) -> bool {
        (px - self.x).abs() <= self.hw && (py - self.y).abs() <= self.hh
    }
}

/// Build every clickable slot. Shared by drawing and hit-testing so they
/// can never drift apart.
pub fn layout(view: &ItemView, bag_open: bool) -> Vec<(SlotRef, HudRect)> {
    let mut out = Vec::new();
    push_row(&mut out, 0, HOTBAR, false, 0.0, HOT_Y, HOT_STEP, HOT_SIZE, SlotRef::Bag);
    if bag_open {
        push_row(
            &mut out,
            HOTBAR,
            BAG_SLOTS - HOTBAR,
            true,
            0.0,
            BAG_Y,
            BAG_STEP,
            BAG_SIZE,
            SlotRef::Bag,
        );
        if let Some(st) = view.open_station_view() {
            if !st.slots.is_empty() {
                let cols = if st.kind == StationKind::Furnace { 3 } else { 6 };
                push_row_cols(
                    &mut out,
                    0,
                    st.slots.len(),
                    cols,
                    ST_X,
                    ST_Y,
                    0.115,
                    0.095,
                    SlotRef::Station,
                );
            }
        }
    }
    out
}

fn push_row(
    out: &mut Vec<(SlotRef, HudRect)>,
    start: usize,
    count: usize,
    wrap: bool,
    ox: f32,
    oy: f32,
    step: f32,
    size: f32,
    mk: fn(usize) -> SlotRef,
) {
    push_row_cols(out, start, count, HOTBAR, ox, oy, step, size, mk);
    let _ = wrap;
}

fn push_row_cols(
    out: &mut Vec<(SlotRef, HudRect)>,
    start: usize,
    count: usize,
    cols: usize,
    ox: f32,
    oy: f32,
    step: f32,
    size: f32,
    mk: fn(usize) -> SlotRef,
) {
    let cols = cols.max(1);
    for i in 0..count {
        let col = i % cols;
        let row = i / cols;
        let x = ox + (col as f32 - (cols as f32 - 1.0) * 0.5) * step;
        let y = oy - row as f32 * step;
        out.push((
            mk(start + i),
            HudRect {
                x,
                y,
                hw: size * 0.5,
                hh: size * 0.5,
            },
        ));
    }
}

/// Which slot is under the mouse, or `None` if the pointer is on empty glass.
pub fn hit_slot(view: &ItemView, bag_open: bool, mx: f32, my: f32) -> Option<SlotRef> {
    layout(view, bag_open)
        .into_iter()
        .rev()
        .find(|(_, r)| r.contains(mx, my))
        .map(|(s, _)| s)
}

/// Produce overlay draws. Order is back-to-front (no depth test):
/// panel → black border → white plate → item gem → count → held → debug.
pub fn hud_draws(
    meshes: &ItemMeshes,
    view: &ItemView,
    ui: &ItemUi,
    debug: Option<&DebugSnap>,
) -> Vec<DrawRequest> {
    let mut out = Vec::new();
    let slots = layout(view, ui.bag_open);

    if ui.bag_open {
        out.push(DrawRequest {
            handle: meshes.slot_border,
            model: card(0.0, 0.08, 1.34, 0.78),
        });
        out.push(DrawRequest {
            handle: meshes.slot_panel,
            model: card(0.0, 0.08, 1.28, 0.72),
        });
        if view.open_station_view().is_some() {
            out.push(DrawRequest {
                handle: meshes.slot_border,
                model: card(ST_X, 0.14, 0.50, 0.62),
            });
            out.push(DrawRequest {
                handle: meshes.slot_panel,
                model: card(ST_X, 0.14, 0.46, 0.58),
            });
        }
    }

    // Slot sprites first so the ResourceBits meshes sit on top.
    for (slot, rect) in &slots {
        let selected = match slot {
            SlotRef::Bag(i) => *i == view.selected,
            SlotRef::Station(i) => ui.focus_station && *i == ui.station_cursor,
        };
        let s = (rect.hw * 2.0) * if selected { 1.08 } else { 1.0 };
        let frame = if selected {
            meshes.slot_sel
        } else if matches!(slot, SlotRef::Station(_)) {
            meshes.slot_station
        } else {
            meshes.slot
        };
        out.push(DrawRequest {
            handle: frame,
            model: sprite(rect.x, rect.y, s, s),
        });
    }

    for (slot, rect) in &slots {
        if ui.hides(*slot) {
            continue;
        }
        let stack = match slot {
            SlotRef::Bag(i) => view.bag.get(*i).copied().unwrap_or_else(Stack::empty),
            SlotRef::Station(i) => view
                .open_station_view()
                .and_then(|s| s.slots.get(*i).copied())
                .unwrap_or_else(Stack::empty),
        };
        if stack.is_empty() {
            continue;
        }
        if let Some(h) = meshes.visual(stack.item, stack.count) {
            let is = if stack.item.def().tool {
                rect.hw * 1.15
            } else {
                rect.hw * 0.95
            };
            out.push(DrawRequest {
                handle: h,
                model: card(rect.x, rect.y + 0.006, is, is),
            });
        }
        push_count(
            &mut out,
            meshes,
            stack.count,
            rect.x + rect.hw * 0.45,
            rect.y - rect.hh * 0.45,
        );
    }

    if ui.bag_open {
        if let Some(recipe) = selected_recipe(view) {
            let mut x = -0.40;
            let y = 0.54;
            for ing in recipe.inputs {
                push_item(&mut out, meshes, (*ing).into(), x, y, 0.07);
                x += 0.10;
            }
            push_item(&mut out, meshes, recipe.output.into(), x + 0.08, y, 0.085);
        }
    }

    if !ui.held.is_empty() {
        let (mx, my) = ui.mouse_ndc;
        push_item(&mut out, meshes, ui.held, mx + 0.04, my - 0.04, 0.09);
    }

    if let Some(dbg) = debug {
        draw_debug(&mut out, meshes, dbg);
    }

    out
}

fn push_item(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, stack: Stack, x: f32, y: f32, size: f32) {
    if stack.is_empty() {
        return;
    }
    let Some(h) = meshes.visual(stack.item, stack.count) else {
        return;
    };
    out.push(DrawRequest {
        handle: h,
        model: card(x, y, size, size),
    });
    push_count(out, meshes, stack.count, x + 0.04, y - 0.04);
}

fn push_count(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, count: u16, x: f32, y: f32) {
    if count <= 1 {
        return;
    }
    let s = format!("{count}");
    let mut ox = 0.0;
    for ch in s.chars() {
        if let Some(d) = ch.to_digit(10) {
            out.push(DrawRequest {
                handle: meshes.digits[d as usize],
                model: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.026, 0.038, 0.026),
                    Quat::IDENTITY,
                    Vec3::new(x + ox, y, 0.0),
                ),
            });
            ox += 0.020;
        }
    }
}

fn draw_debug(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, d: &DebugSnap) {
    let held = if d.held.is_empty() {
        "NONE".to_string()
    } else {
        format!("{} X{}", d.held.item.def().name.to_ascii_uppercase(), d.held.count)
    };
    let lines = [
        format!("FPS {:.0}", d.fps),
        format!(
            "POS {:.2} {:.2} {:.2}",
            d.pos.x, d.pos.y, d.pos.z
        ),
        format!("VEL {:.2} {:.2} {:.2}", d.vel.x, d.vel.y, d.vel.z),
        format!("YAW {:.2} PITCH {:.2}", d.yaw, d.pitch),
        format!(
            "GROUND {} SIT {}",
            if d.grounded { "Y" } else { "N" },
            if d.sitting { "Y" } else { "N" }
        ),
        format!(
            "BAG {} SEL {} HELD {}",
            if d.bag_open { "OPEN" } else { "HOT" },
            d.selected,
            held
        ),
        format!(
            "STATION {} LOOT {} {}",
            d.station.unwrap_or("NONE"),
            d.loot,
            if d.multiplayer { "MP" } else { "SP" }
        ),
    ];
    let mut y = 0.90;
    for line in lines {
        draw_text(out, meshes, &line, -0.96, y, 0.028);
        y -= 0.045;
    }
}

fn draw_text(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, text: &str, x: f32, y: f32, size: f32) {
    let mut ox = 0.0;
    for ch in text.chars() {
        let key = if ch == ' ' { ' ' } else { ch.to_ascii_uppercase() };
        if let Some(&h) = meshes.glyphs.get(&key) {
            out.push(DrawRequest {
                handle: h,
                model: Mat4::from_scale_rotation_translation(
                    Vec3::new(size * 0.72, size, size),
                    Quat::IDENTITY,
                    Vec3::new(x + ox, y, 0.0),
                ),
            });
        }
        ox += size * 0.72;
    }
}

/// XZ-flat plate/cube → screen card at `(x, y)` in NDC (3D ResourceBits).
fn card(x: f32, y: f32, w: f32, h: f32) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::new(w, 0.03, h),
        Quat::from_rotation_x(FACE_CAM),
        Vec3::new(x, y, 0.0),
    )
}

/// InventorySlotsSet sprite (already faces +Z) placed in NDC.
fn sprite(x: f32, y: f32, w: f32, h: f32) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::new(w, h, 1.0),
        Quat::IDENTITY,
        Vec3::new(x, y, 0.0),
    )
}

pub fn status_line(view: &ItemView, ui: &ItemUi, prompt: Option<&str>) -> String {
    let sel = view.selected_stack();
    let held = if !ui.held.is_empty() {
        format!("held {} x{}", ui.held.item.def().name, ui.held.count)
    } else if sel.is_empty() {
        "empty".to_string()
    } else {
        format!("{} x{}", sel.item.def().name, sel.count)
    };
    let recipe = selected_recipe(view)
        .map(|r| format!("[R] {}", r.name))
        .unwrap_or_default();
    let open = if view.open_station.is_some() {
        "STATION"
    } else if ui.bag_open {
        "BAG"
    } else {
        "hotbar"
    };
    let p = prompt.unwrap_or("");
    let log = if view.last_log.is_empty() {
        String::new()
    } else {
        format!(" · {}", view.last_log)
    };
    format!("{open}  {held}  {recipe}  {p}{log}")
}
