//! Screen-space inventory HUD — RPG bag, stats, skills, craft, build.
//!
//! Overlay camera is identity + Y-flip, so these positions are NDC and never
//! follow the chase cam.

use std::collections::HashMap;

use anyhow::Result;
use glam::{Mat4, Quat, Vec3};

use crate::assets::{
    digit_quad, glyph_quad, item_gem, load_rgba_png, material_lib, resolve_asset, slot_plate,
    sprite_quad,
};
use crate::items::{
    selected_recipe, EquipKind, InvTab, ItemId, ItemUi, ItemView, SlotRef, Stack, StationKind,
    BAG_SLOTS, CATALOG, HOTBAR, RESOURCE_BITS_DIR,
};
use crate::rpg::{equip_label, StatId, EQUIP_SLOTS};
use crate::scene::DrawRequest;
use crate::vulkan::{ModelHandle, VulkanContext};

/// Rotate the XZ-flat slot plate so its face is toward the overlay camera (−Z).
const FACE_CAM: f32 = std::f32::consts::FRAC_PI_2;

const HOT_Y: f32 = -0.88;
const HOT_STEP: f32 = 0.138;
const HOT_SIZE: f32 = 0.112;
const BAG_X: f32 = -0.06;
const BAG_Y: f32 = 0.38;
const BAG_STEP: f32 = 0.128;
const BAG_SIZE: f32 = 0.108;
const EQ_X: f32 = -0.78;
const EQ_Y: f32 = 0.40;
const EQ_STEP: f32 = 0.145;
const ST_X: f32 = 0.72;
const ST_Y: f32 = 0.28;
const TAB_Y: f32 = 0.74;
const TAB_STEP: f32 = 0.30;
const TAB_W: f32 = 0.26;

pub const GLYPHS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ .:-+/_()[]=%,'?!*";

pub struct ItemMeshes {
    pub by_item: HashMap<u16, ModelHandle>,
    pub by_mesh: HashMap<String, ModelHandle>,
    pub slot: ModelHandle,
    pub slot_sel: ModelHandle,
    pub slot_station: ModelHandle,
    pub slot_panel: ModelHandle,
    pub slot_border: ModelHandle,
    pub panel_dark: ModelHandle,
    pub panel_gold: ModelHandle,
    pub dim: ModelHandle,
    pub bar_bg: ModelHandle,
    pub bar_hp: ModelHandle,
    pub bar_xp: ModelHandle,
    pub digits: [ModelHandle; 10],
    pub glyphs: HashMap<char, ModelHandle>,
    pub chest_body: ModelHandle,
    pub chest_lid: ModelHandle,
    pub furnace: ModelHandle,
    pub ember: ModelHandle,
    pub workbench: ModelHandle,
    pub block_by_item: HashMap<u16, ModelHandle>,
}

impl ItemMeshes {
    pub fn upload(
        vk: &mut VulkanContext,
        chest_body: ModelHandle,
        chest_lid: ModelHandle,
        furnace: ModelHandle,
        ember: ModelHandle,
        workbench: ModelHandle,
        mats: &material_lib::MatCache,
    ) -> Result<Self> {
        let mut by_item = HashMap::new();
        let mut by_mesh = HashMap::new();
        let mut block_by_item = HashMap::new();
        for def in CATALOG {
            if let Some(px) = mats.named(def.mat) {
                by_item.insert(
                    def.id.0,
                    vk.upload_model(&material_lib::item_icon(px.clone(), def.name))?,
                );
            } else {
                by_item.insert(def.id.0, vk.upload_model(&item_gem(def.color))?);
            }
            if def.place != 0 {
                if let Some(px) = mats.named(def.mat) {
                    block_by_item.insert(
                        def.id.0,
                        vk.upload_model(&material_lib::textured_box(1.0, 1.0, 1.0, px.clone(), def.name))?,
                    );
                }
            }
            if !def.held.is_empty() && !by_mesh.contains_key(def.held) {
                match crate::assets::load_gltf(resolve_asset(def.held)) {
                    Ok(model) => match vk.upload_model(&model) {
                        Ok(h) => {
                            by_mesh.insert(def.held.to_string(), h);
                        }
                        Err(e) => log::warn!("upload held {}: {e:#}", def.held),
                    },
                    Err(e) => log::warn!("held {}: {e:#}", def.held),
                }
            }
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
        for ch in GLYPHS.chars() {
            glyphs.insert(ch, vk.upload_model(&glyph_quad(ch))?);
        }
        Ok(Self {
            by_item,
            by_mesh,
            slot: load_slot(vk, "wood")?,
            slot_sel: load_slot(vk, "orange_red")?,
            slot_station: load_slot(vk, "coldsteel")?,
            slot_panel: vk.upload_model(&slot_plate([1.0, 1.0, 1.0, 0.18], "panel"))?,
            slot_border: vk.upload_model(&slot_plate([0.08, 0.06, 0.04, 0.72], "border"))?,
            panel_dark: vk.upload_model(&slot_plate([0.07, 0.06, 0.05, 0.92], "dark"))?,
            panel_gold: vk.upload_model(&slot_plate([0.72, 0.58, 0.28, 0.85], "gold"))?,
            dim: vk.upload_model(&slot_plate([0.02, 0.02, 0.03, 0.62], "dim"))?,
            bar_bg: vk.upload_model(&slot_plate([0.08, 0.08, 0.09, 0.9], "barbg"))?,
            bar_hp: vk.upload_model(&slot_plate([0.72, 0.18, 0.16, 0.95], "barhp"))?,
            bar_xp: vk.upload_model(&slot_plate([0.28, 0.52, 0.78, 0.95], "barxp"))?,
            digits,
            glyphs,
            chest_body,
            chest_lid,
            furnace,
            ember,
            workbench,
            block_by_item,
        })
    }

    pub fn item(&self, id: ItemId) -> Option<ModelHandle> {
        self.visual(id, 1)
    }

    pub fn visual(&self, id: ItemId, count: u16) -> Option<ModelHandle> {
        let def = id.def();
        if !def.held.is_empty() {
            if let Some(h) = self.by_mesh.get(def.held) {
                return Some(*h);
            }
        }
        let stem = id.visual_mesh(count);
        if !stem.is_empty() {
            if let Some(h) = self.by_mesh.get(stem) {
                return Some(*h);
            }
        }
        self.by_item.get(&id.0).copied()
    }

    pub fn block_of(&self, id: ItemId) -> Option<ModelHandle> {
        self.block_by_item.get(&id.0).copied().or_else(|| self.visual(id, 1))
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

#[derive(Clone, Copy)]
pub struct HudRect {
    pub x: f32,
    pub y: f32,
    pub hw: f32,
    pub hh: f32,
}

impl HudRect {
    pub fn contains(self, px: f32, py: f32) -> bool {
        (px - self.x).abs() <= self.hw && (py - self.y).abs() <= self.hh
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudHit {
    Slot(SlotRef),
    Tab(InvTab),
    StatPlus(u8),
    Craft,
    Recipe(i32),
    SelectBag(usize),
}

pub fn layout(view: &ItemView, ui: &ItemUi) -> Vec<(SlotRef, HudRect)> {
    let mut out = Vec::new();
    push_row(
        &mut out,
        0,
        HOTBAR,
        0.0,
        HOT_Y,
        HOT_STEP,
        HOT_SIZE,
        SlotRef::Bag,
    );
    if !ui.bag_open {
        return out;
    }
    if ui.tab == InvTab::Bag {
        push_row_cols(
            &mut out,
            HOTBAR,
            BAG_SLOTS - HOTBAR,
            HOTBAR,
            BAG_X,
            BAG_Y,
            BAG_STEP,
            BAG_SIZE,
            SlotRef::Bag,
        );
        for i in 0..EQUIP_SLOTS {
            out.push((
                SlotRef::Equip(i),
                HudRect {
                    x: EQ_X,
                    y: EQ_Y - i as f32 * EQ_STEP,
                    hw: BAG_SIZE * 0.5,
                    hh: BAG_SIZE * 0.5,
                },
            ));
        }
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
                    0.118,
                    0.100,
                    SlotRef::Station,
                );
            }
        }
    }
    if ui.tab == InvTab::Build {
        // Placeable items still live in the bag — expose the hotbar only.
    }
    out
}

fn push_row(
    out: &mut Vec<(SlotRef, HudRect)>,
    start: usize,
    count: usize,
    ox: f32,
    oy: f32,
    step: f32,
    size: f32,
    mk: fn(usize) -> SlotRef,
) {
    push_row_cols(out, start, count, HOTBAR, ox, oy, step, size, mk);
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

pub fn tab_rects() -> Vec<(InvTab, HudRect)> {
    InvTab::ALL
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let x = -0.60 + i as f32 * TAB_STEP;
            (
                *t,
                HudRect {
                    x,
                    y: TAB_Y,
                    hw: TAB_W * 0.5,
                    hh: 0.045,
                },
            )
        })
        .collect()
}

pub fn hit(view: &ItemView, ui: &ItemUi, mx: f32, my: f32) -> Option<HudHit> {
    if ui.bag_open {
        for (t, r) in tab_rects() {
            if r.contains(mx, my) {
                return Some(HudHit::Tab(t));
            }
        }
        match ui.tab {
            InvTab::Stats => {
                for (i, _) in StatId::ALL.iter().enumerate() {
                    let y = 0.38 - i as f32 * 0.13;
                    let r = HudRect {
                        x: 0.42,
                        y,
                        hw: 0.04,
                        hh: 0.04,
                    };
                    if r.contains(mx, my) {
                        return Some(HudHit::StatPlus(i as u8));
                    }
                }
            }
            InvTab::Craft => {
                let craft = HudRect {
                    x: 0.52,
                    y: -0.28,
                    hw: 0.16,
                    hh: 0.05,
                };
                if craft.contains(mx, my) {
                    return Some(HudHit::Craft);
                }
                let list: Vec<_> = craft_list(view);
                for (i, _) in list.iter().enumerate() {
                    let y = 0.42 - i as f32 * 0.07;
                    let r = HudRect {
                        x: -0.42,
                        y,
                        hw: 0.36,
                        hh: 0.03,
                    };
                    if r.contains(mx, my) {
                        return Some(HudHit::Recipe(i as i32));
                    }
                }
            }
            InvTab::Build => {
                for (idx, rect, _) in build_palette(view) {
                    if rect.contains(mx, my) {
                        return Some(HudHit::SelectBag(idx));
                    }
                }
            }
            _ => {}
        }
    }
    layout(view, ui)
        .into_iter()
        .rev()
        .find(|(_, r)| r.contains(mx, my))
        .map(|(s, _)| HudHit::Slot(s))
}

pub fn hit_slot(view: &ItemView, ui: &ItemUi, mx: f32, my: f32) -> Option<SlotRef> {
    match hit(view, ui, mx, my) {
        Some(HudHit::Slot(s)) => Some(s),
        _ => None,
    }
}

fn craft_list(view: &ItemView) -> Vec<&'static crate::items::Recipe> {
    let at = view
        .open_station
        .and_then(|id| view.stations.iter().find(|s| s.id == id))
        .and_then(|s| s.kind.craft_station())
        .unwrap_or(crate::items::CraftStation::Hand);
    crate::items::recipes_for(at).collect()
}

fn build_palette(view: &ItemView) -> Vec<(usize, HudRect, Stack)> {
    let mut out = Vec::new();
    let mut col = 0;
    let mut row = 0;
    for (idx, stack) in view.bag.iter().enumerate() {
        if stack.is_empty() || stack.item.def().place == 0 {
            continue;
        }
        let x = -0.55 + col as f32 * 0.22;
        let y = 0.28 - row as f32 * 0.22;
        out.push((
            idx,
            HudRect {
                x,
                y,
                hw: 0.08,
                hh: 0.08,
            },
            *stack,
        ));
        col += 1;
        if col >= 6 {
            col = 0;
            row += 1;
        }
    }
    out
}

pub fn hud_draws(
    meshes: &ItemMeshes,
    view: &ItemView,
    ui: &ItemUi,
    debug: Option<&DebugSnap>,
) -> Vec<DrawRequest> {
    let mut out = Vec::new();
    draw_vitals(&mut out, meshes, view);

    if ui.bag_open {
        out.push(DrawRequest {
            handle: meshes.dim,
            model: sprite(0.0, 0.05, 2.1, 1.55),
        });
        out.push(DrawRequest {
            handle: meshes.slot_border,
            model: card(0.0, 0.08, 1.86, 1.42),
        });
        out.push(DrawRequest {
            handle: meshes.panel_dark,
            model: card(0.0, 0.08, 1.78, 1.34),
        });
        draw_tabs(&mut out, meshes, ui.tab);
        match ui.tab {
            InvTab::Bag => draw_bag(&mut out, meshes, view, ui),
            InvTab::Stats => draw_stats(&mut out, meshes, view),
            InvTab::Skills => draw_skills(&mut out, meshes, view),
            InvTab::Craft => draw_craft(&mut out, meshes, view),
            InvTab::Build => draw_build(&mut out, meshes, view),
        }
    }

    // Hotbar always sits on top of the world (and under the bag panel's lower edge).
    draw_slots(&mut out, meshes, view, ui, true);

    if !ui.held.is_empty() {
        let (mx, my) = ui.mouse_ndc;
        push_item(&mut out, meshes, ui.held, mx + 0.04, my - 0.04, 0.09);
    }

    if let Some(slot) = ui.hover {
        if let Some(tip) = tooltip(view, slot) {
            let (mx, my) = ui.mouse_ndc;
            draw_tooltip(&mut out, meshes, &tip, mx + 0.16, my + 0.08);
        }
    }

    if let Some(dbg) = debug {
        draw_debug(&mut out, meshes, dbg);
    }

    out
}

fn draw_vitals(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, view: &ItemView) {
    let hp = (view.hero.hp / view.hero.max_hp()).clamp(0.0, 1.0);
    let x = -0.78;
    let y = -0.72;
    out.push(DrawRequest {
        handle: meshes.bar_bg,
        model: card(x, y, 0.36, 0.045),
    });
    out.push(DrawRequest {
        handle: meshes.bar_hp,
        model: card(x - 0.18 * (1.0 - hp), y, 0.36 * hp, 0.032),
    });
    draw_text(
        out,
        meshes,
        &format!("HP {:.0}/{:.0}", view.hero.hp, view.hero.max_hp()),
        x - 0.16,
        y + 0.04,
        0.018,
    );
    let xr = (view.hero.xp as f32 / view.hero.next_level() as f32).clamp(0.0, 1.0);
    out.push(DrawRequest {
        handle: meshes.bar_bg,
        model: card(x, y - 0.06, 0.36, 0.028),
    });
    out.push(DrawRequest {
        handle: meshes.bar_xp,
        model: card(x - 0.18 * (1.0 - xr), y - 0.06, 0.36 * xr, 0.018),
    });
    draw_text(
        out,
        meshes,
        &format!("LV {}", view.hero.level),
        x - 0.16,
        y - 0.095,
        0.016,
    );
}

fn draw_tabs(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, active: InvTab) {
    for (t, rect) in tab_rects() {
        let on = t == active;
        let frame = if on { meshes.slot_sel } else { meshes.slot };
        out.push(DrawRequest {
            handle: frame,
            model: sprite(rect.x, rect.y, rect.hw * 2.0, rect.hh * 2.15),
        });
        let size = 0.022;
        let w = t.label().len() as f32 * size * 0.72;
        draw_text(out, meshes, t.label(), rect.x - w * 0.5, rect.y, size);
    }
}

fn draw_slots(
    out: &mut Vec<DrawRequest>,
    meshes: &ItemMeshes,
    view: &ItemView,
    ui: &ItemUi,
    hotbar_only: bool,
) {
    let slots = layout(view, ui);
    for (slot, rect) in &slots {
        if hotbar_only {
            if !matches!(slot, SlotRef::Bag(i) if *i < HOTBAR) {
                continue;
            }
        } else if matches!(slot, SlotRef::Bag(i) if *i < HOTBAR) {
            continue;
        }
        let selected = match slot {
            SlotRef::Bag(i) => *i == view.selected,
            SlotRef::Station(i) => ui.focus_station && *i == ui.station_cursor,
            SlotRef::Equip(_) => false,
        };
        let s = (rect.hw * 2.0) * if selected { 1.08 } else { 1.0 };
        let frame = if selected {
            meshes.slot_sel
        } else if matches!(slot, SlotRef::Station(_) | SlotRef::Equip(_)) {
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
        if hotbar_only {
            if !matches!(slot, SlotRef::Bag(i) if *i < HOTBAR) {
                continue;
            }
        } else if matches!(slot, SlotRef::Bag(i) if *i < HOTBAR) {
            continue;
        }
        let stack = match slot {
            SlotRef::Bag(i) => view.bag.get(*i).copied().unwrap_or_else(Stack::empty),
            SlotRef::Station(i) => view
                .open_station_view()
                .and_then(|s| s.slots.get(*i).copied())
                .unwrap_or_else(Stack::empty),
            SlotRef::Equip(i) => view.equip.get(*i).copied().unwrap_or_else(Stack::empty),
        };
        if stack.is_empty() {
            continue;
        }
        if let Some(h) = meshes.visual(stack.item, stack.count) {
            let is = if stack.item.def().tool {
                rect.hw * 1.15
            } else {
                rect.hw * 0.92
            };
            out.push(DrawRequest {
                handle: h,
                model: card(rect.x, rect.y + 0.006, is, is),
            });
        }
        push_count(
            out,
            meshes,
            stack.count,
            rect.x + rect.hw * 0.42,
            rect.y - rect.hh * 0.42,
        );
    }
}

fn draw_bag(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, view: &ItemView, ui: &ItemUi) {
    draw_text(out, meshes, "EQUIPMENT", EQ_X - 0.10, EQ_Y + 0.12, 0.018);
    for i in 0..EQUIP_SLOTS {
        let y = EQ_Y - i as f32 * EQ_STEP;
        draw_text(out, meshes, equip_label(i), EQ_X - 0.22, y + 0.07, 0.014);
    }
    draw_text(out, meshes, "PACK", BAG_X - 0.52, BAG_Y + 0.12, 0.018);
    draw_slots(out, meshes, view, ui, false);

    if let Some(st) = view.open_station_view() {
        draw_text(out, meshes, st.kind.name(), ST_X - 0.16, 0.52, 0.020);
        if st.kind == StationKind::Furnace {
            draw_text(out, meshes, "IN  FUEL  OUT", ST_X - 0.18, ST_Y + 0.10, 0.014);
        }
    }

    let inspect = ui
        .hover
        .map(|s| peek(view, s))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let s = view.selected_stack();
            (!s.is_empty()).then_some(s)
        });
    if let Some(stack) = inspect {
        let def = stack.item.def();
        out.push(DrawRequest {
            handle: meshes.slot_border,
            model: card(0.62, -0.22, 0.46, 0.42),
        });
        out.push(DrawRequest {
            handle: meshes.slot_panel,
            model: card(0.62, -0.22, 0.42, 0.38),
        });
        draw_text(out, meshes, &def.name.to_ascii_uppercase(), 0.44, -0.08, 0.020);
        draw_text(out, meshes, def.desc, 0.44, -0.14, 0.014);
        draw_text(
            out,
            meshes,
            &format!("x{}  STACK {}", stack.count, def.stack),
            0.44,
            -0.20,
            0.014,
        );
        if def.equip != EquipKind::None {
            draw_text(out, meshes, def.equip.label(), 0.44, -0.26, 0.014);
        }
        if def.place != 0 {
            draw_text(out, meshes, "PLACEABLE  RMB IN WORLD", 0.44, -0.32, 0.014);
        }
    }
}

fn draw_stats(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, view: &ItemView) {
    let h = &view.hero;
    draw_text(out, meshes, &format!("ADVENTURER  LV {}", h.level), -0.70, 0.58, 0.028);
    draw_text(
        out,
        meshes,
        &format!("UNSPENT POINTS  {}", h.unspent),
        -0.70,
        0.50,
        0.020,
    );
    for (i, stat) in StatId::ALL.iter().enumerate() {
        let y = 0.38 - i as f32 * 0.13;
        let val = h.stat(*stat);
        draw_text(out, meshes, stat.label(), -0.70, y + 0.04, 0.020);
        draw_text(out, meshes, stat.hint(), -0.70, y - 0.02, 0.014);
        let ratio = val as f32 / 20.0;
        out.push(DrawRequest {
            handle: meshes.bar_bg,
            model: card(-0.10, y, 0.42, 0.04),
        });
        out.push(DrawRequest {
            handle: meshes.bar_xp,
            model: card(-0.10 - 0.21 * (1.0 - ratio), y, 0.42 * ratio, 0.028),
        });
        draw_text(out, meshes, &format!("{}", val), 0.18, y, 0.022);
        out.push(DrawRequest {
            handle: if h.unspent > 0 {
                meshes.slot_sel
            } else {
                meshes.slot_station
            },
            model: sprite(0.42, y, 0.08, 0.08),
        });
        draw_text(out, meshes, "+", 0.405, y, 0.028);
    }
    draw_text(
        out,
        meshes,
        &format!(
            "HP {:.0}  STAM {:.0}  DMG {:.0}  CARRY {}  SPEED {:.2}",
            h.max_hp(),
            h.max_stam(),
            h.melee(),
            h.carry(),
            h.speed_mult()
        ),
        -0.70,
        -0.32,
        0.016,
    );
}

fn draw_skills(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, view: &ItemView) {
    draw_text(out, meshes, "TRADES", -0.70, 0.58, 0.028);
    for (i, skill) in view.hero.skills.iter().enumerate() {
        let y = 0.42 - i as f32 * 0.16;
        draw_text(out, meshes, skill.id.name(), -0.70, y + 0.04, 0.022);
        draw_text(out, meshes, skill.id.hint(), -0.70, y - 0.02, 0.014);
        draw_text(out, meshes, &format!("LV {}", skill.level), 0.42, y + 0.04, 0.020);
        let r = skill.ratio();
        out.push(DrawRequest {
            handle: meshes.bar_bg,
            model: card(-0.08, y - 0.04, 0.70, 0.036),
        });
        out.push(DrawRequest {
            handle: meshes.bar_xp,
            model: card(-0.08 - 0.35 * (1.0 - r), y - 0.04, 0.70 * r, 0.024),
        });
        draw_text(
            out,
            meshes,
            &format!("{} / {}", skill.xp, skill.next()),
            0.22,
            y - 0.04,
            0.014,
        );
    }
}

fn draw_craft(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, view: &ItemView) {
    let list = craft_list(view);
    draw_text(out, meshes, "RECIPES", -0.70, 0.58, 0.026);
    draw_text(out, meshes, "[ ] TO CYCLE   R OR CRAFT TO MAKE", -0.70, 0.52, 0.014);
    for (i, recipe) in list.iter().enumerate() {
        let y = 0.42 - i as f32 * 0.07;
        let on = i == view.recipe_cursor % list.len().max(1);
        if on {
            out.push(DrawRequest {
                handle: meshes.slot_sel,
                model: sprite(-0.42, y, 0.72, 0.06),
            });
        }
        draw_text(out, meshes, recipe.name, -0.72, y, 0.018);
    }
    if let Some(recipe) = selected_recipe(view) {
        out.push(DrawRequest {
            handle: meshes.slot_border,
            model: card(0.52, 0.10, 0.58, 0.70),
        });
        out.push(DrawRequest {
            handle: meshes.slot_panel,
            model: card(0.52, 0.10, 0.54, 0.66),
        });
        draw_text(out, meshes, recipe.name, 0.28, 0.36, 0.020);
        let mut y = 0.26;
        for ing in recipe.inputs {
            let have = view
                .bag
                .iter()
                .filter(|s| s.item == ing.item)
                .map(|s| s.count)
                .sum::<u16>();
            push_item(out, meshes, (*ing).into(), 0.30, y, 0.07);
            draw_text(
                out,
                meshes,
                &format!("{}  {}/{}", ing.item.def().name, have, ing.count),
                0.38,
                y,
                0.016,
            );
            y -= 0.10;
        }
        draw_text(out, meshes, "MAKES", 0.28, y, 0.016);
        push_item(out, meshes, recipe.output.into(), 0.42, y - 0.10, 0.09);
        out.push(DrawRequest {
            handle: meshes.slot_sel,
            model: sprite(0.52, -0.28, 0.32, 0.10),
        });
        draw_text(out, meshes, "CRAFT", 0.42, -0.28, 0.024);
    }
}

fn draw_build(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, view: &ItemView) {
    draw_text(out, meshes, "BUILDING", -0.70, 0.58, 0.028);
    draw_text(
        out,
        meshes,
        "CLICK A BLOCK THEN RMB IN THE WORLD TO PLACE",
        -0.70,
        0.50,
        0.016,
    );
    draw_text(
        out,
        meshes,
        "HOLD A TOOL AND PRESS ATTACK TO REMOVE",
        -0.70,
        0.44,
        0.016,
    );
    for (idx, rect, stack) in build_palette(view) {
        let on = view.selected == idx;
        let frame = if on { meshes.slot_sel } else { meshes.slot };
        out.push(DrawRequest {
            handle: frame,
            model: sprite(rect.x, rect.y, rect.hw * 2.0, rect.hh * 2.0),
        });
        push_item(out, meshes, stack, rect.x, rect.y, 0.10);
        draw_text(
            out,
            meshes,
            stack.item.def().name,
            rect.x - 0.07,
            rect.y - 0.10,
            0.012,
        );
    }
    let sel = view.selected_stack();
    if sel.item.def().place != 0 {
        draw_text(
            out,
            meshes,
            &format!("READY  {}", sel.item.def().name.to_ascii_uppercase()),
            -0.70,
            -0.36,
            0.020,
        );
    } else {
        draw_text(out, meshes, "NO PLACEABLE SELECTED", -0.70, -0.36, 0.018);
    }
}

fn peek(view: &ItemView, slot: SlotRef) -> Stack {
    match slot {
        SlotRef::Bag(i) => view.bag.get(i).copied().unwrap_or_else(Stack::empty),
        SlotRef::Station(i) => view
            .open_station_view()
            .and_then(|s| s.slots.get(i).copied())
            .unwrap_or_else(Stack::empty),
        SlotRef::Equip(i) => view.equip.get(i).copied().unwrap_or_else(Stack::empty),
    }
}

fn tooltip(view: &ItemView, slot: SlotRef) -> Option<String> {
    let stack = peek(view, slot);
    if stack.is_empty() {
        return match slot {
            SlotRef::Equip(i) => Some(equip_label(i).to_string()),
            _ => None,
        };
    }
    let def = stack.item.def();
    Some(format!("{} x{}", def.name.to_ascii_uppercase(), stack.count))
}

fn draw_tooltip(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, text: &str, x: f32, y: f32) {
    let w = (text.len() as f32 * 0.014).max(0.20);
    out.push(DrawRequest {
        handle: meshes.panel_dark,
        model: card(x, y, w + 0.06, 0.07),
    });
    draw_text(out, meshes, text, x - w * 0.5, y, 0.016);
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
        format!("POS {:.2} {:.2} {:.2}", d.pos.x, d.pos.y, d.pos.z),
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

pub fn draw_text(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, text: &str, x: f32, y: f32, size: f32) {
    let mut ox = 0.0;
    for ch in text.chars() {
        let key = match ch {
            'a'..='z' => ch.to_ascii_uppercase(),
            _ => ch,
        };
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

pub fn card(x: f32, y: f32, w: f32, h: f32) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::new(w, 0.03, h),
        Quat::from_rotation_x(FACE_CAM),
        Vec3::new(x, y, 0.0),
    )
}

pub fn sprite(x: f32, y: f32, w: f32, h: f32) -> Mat4 {
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
        ui.tab.label()
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
