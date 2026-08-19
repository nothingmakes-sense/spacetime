//! Camera-locked inventory / hotbar / station / recipe HUD.
//!
//! Drawn with the existing Phong pipeline as small colored cubes so we do
//! not pull in a second UI library. Slot counts use pre-uploaded digit quads.

use std::collections::HashMap;

use anyhow::Result;
use glam::{Mat4, Vec3};

use crate::assets::{digit_quad, item_gem, slot_plate};
use crate::items::{
    selected_recipe, ItemId, ItemUi, ItemView, Stack, StationKind, BAG_SLOTS, CATALOG, HOTBAR,
};
use crate::scene::DrawRequest;
use crate::vulkan::{ModelHandle, VulkanContext};

pub struct ItemMeshes {
    pub by_item: HashMap<u16, ModelHandle>,
    pub slot: ModelHandle,
    pub slot_sel: ModelHandle,
    pub slot_panel: ModelHandle,
    pub digits: [ModelHandle; 10],
    pub chest_body: ModelHandle,
    pub chest_lid: ModelHandle,
    pub furnace: ModelHandle,
    pub ember: ModelHandle,
    pub workbench: ModelHandle,
}

impl ItemMeshes {
    pub fn upload(
        vk: &mut VulkanContext,
        chest_body: ModelHandle,
        chest_lid: ModelHandle,
        furnace: ModelHandle,
        ember: ModelHandle,
        workbench: ModelHandle,
    ) -> Result<Self> {
        let mut by_item = HashMap::new();
        for def in CATALOG {
            by_item.insert(def.id.0, vk.upload_model(&item_gem(def.color))?);
        }
        let mut digits = [ModelHandle(0); 10];
        for d in 0..10u8 {
            digits[d as usize] = vk.upload_model(&digit_quad(d))?;
        }
        Ok(Self {
            by_item,
            slot: vk.upload_model(&slot_plate([0.10, 0.10, 0.12, 1.0], "slot"))?,
            slot_sel: vk.upload_model(&slot_plate([0.85, 0.72, 0.22, 1.0], "slot_sel"))?,
            slot_panel: vk.upload_model(&slot_plate([0.06, 0.06, 0.07, 1.0], "panel"))?,
            digits,
            chest_body,
            chest_lid,
            furnace,
            ember,
            workbench,
        })
    }

    pub fn item(&self, id: ItemId) -> Option<ModelHandle> {
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

pub fn hud_draws(meshes: &ItemMeshes, view: &ItemView, ui: &ItemUi, eye: Vec3, view_mat: Mat4) -> Vec<DrawRequest> {
    let inv = view_mat.inverse();
    let right = inv.transform_vector3(Vec3::X).normalize_or_zero();
    let up = inv.transform_vector3(Vec3::Y).normalize_or_zero();
    let fwd = -inv.transform_vector3(Vec3::Z).normalize_or_zero();
    let origin = eye + fwd * 1.55;

    let mut out = Vec::new();

    // Hotbar — always visible along the bottom of the view.
    let hot_y = -0.42;
    draw_slot_row(
        &mut out,
        meshes,
        &view.bag,
        0,
        HOTBAR,
        view.selected,
        origin,
        right,
        up,
        0.0,
        hot_y,
        0.095,
        false,
    );

    if ui.bag_open {
        // Dark panel behind the bag grid.
        let panel = origin + up * 0.06 + fwd * -0.04;
        out.push(DrawRequest {
            handle: meshes.slot_panel,
            model: basis(panel, right, up, fwd, 1.05, 0.01, 0.72),
        });
        draw_slot_row(
            &mut out,
            meshes,
            &view.bag,
            HOTBAR,
            BAG_SLOTS - HOTBAR,
            view.selected,
            origin,
            right,
            up,
            0.0,
            0.18,
            0.095,
            true,
        );

        if let Some(st) = view.open_station_view() {
            if !st.slots.is_empty() {
                let ox = 0.62;
                draw_slot_row(
                    &mut out,
                    meshes,
                    &st.slots,
                    0,
                    st.slots.len(),
                    if ui.focus_station {
                        ui.station_cursor
                    } else {
                        usize::MAX
                    },
                    origin,
                    right,
                    up,
                    ox,
                    0.22,
                    0.09,
                    true,
                );
            }
        }

        if let Some(recipe) = selected_recipe(view) {
            let mut x = -0.38;
            let y = 0.46;
            for ing in recipe.inputs {
                push_item(&mut out, meshes, (*ing).into(), origin, right, up, fwd, x, y, 0.07);
                x += 0.10;
            }
            push_item(
                &mut out,
                meshes,
                recipe.output.into(),
                origin,
                right,
                up,
                fwd,
                x + 0.06,
                y,
                0.08,
            );
        }
    }

    out
}

fn draw_slot_row(
    out: &mut Vec<DrawRequest>,
    meshes: &ItemMeshes,
    slots: &[Stack],
    start: usize,
    count: usize,
    selected: usize,
    origin: Vec3,
    right: Vec3,
    up: Vec3,
    ox: f32,
    oy: f32,
    step: f32,
    wrap: bool,
) {
    let cols = HOTBAR;
    for i in 0..count {
        let idx = start + i;
        let col = if wrap { i % cols } else { i };
        let row = if wrap { i / cols } else { 0 };
        let x = ox + (col as f32 - (cols as f32 - 1.0) * 0.5) * step;
        let y = oy - row as f32 * step;
        let pos = origin + right * x + up * y;
        let sel = idx == selected;
        let frame = if sel { meshes.slot_sel } else { meshes.slot };
        let s = if sel { 0.092 } else { 0.082 };
        out.push(DrawRequest {
            handle: frame,
            model: Mat4::from_scale_rotation_translation(
                Vec3::new(s, s * 0.35, s),
                glam::Quat::IDENTITY,
                pos,
            ),
        });
        if let Some(stack) = slots.get(idx) {
            if !stack.is_empty() {
                if let Some(h) = meshes.item(stack.item) {
                    out.push(DrawRequest {
                        handle: h,
                        model: Mat4::from_scale_rotation_translation(
                            Vec3::splat(if stack.item.def().tool { 0.055 } else { 0.045 }),
                            glam::Quat::IDENTITY,
                            pos + up * 0.018,
                        ),
                    });
                }
                push_count(out, meshes, stack.count, pos + right * 0.028 + up * -0.028, right, up);
            }
        }
    }
}

fn push_item(
    out: &mut Vec<DrawRequest>,
    meshes: &ItemMeshes,
    stack: Stack,
    origin: Vec3,
    right: Vec3,
    up: Vec3,
    _fwd: Vec3,
    x: f32,
    y: f32,
    size: f32,
) {
    if stack.is_empty() {
        return;
    }
    let Some(h) = meshes.item(stack.item) else {
        return;
    };
    let pos = origin + right * x + up * y;
    out.push(DrawRequest {
        handle: h,
        model: Mat4::from_scale_rotation_translation(Vec3::splat(size), glam::Quat::IDENTITY, pos),
    });
    push_count(out, meshes, stack.count, pos + right * 0.03 + up * -0.03, right, up);
}

fn push_count(out: &mut Vec<DrawRequest>, meshes: &ItemMeshes, count: u16, pos: Vec3, right: Vec3, up: Vec3) {
    if count <= 1 {
        return;
    }
    let s = format!("{count}");
    let mut x = 0.0;
    for ch in s.chars() {
        if let Some(d) = ch.to_digit(10) {
            out.push(DrawRequest {
                handle: meshes.digits[d as usize],
                model: Mat4::from_scale_rotation_translation(
                    Vec3::new(0.022, 0.032, 0.022),
                    glam::Quat::IDENTITY,
                    pos + right * x,
                ),
            });
            let _ = up;
            x += 0.018;
        }
    }
}

fn basis(pos: Vec3, right: Vec3, up: Vec3, fwd: Vec3, sx: f32, sy: f32, sz: f32) -> Mat4 {
    Mat4::from_cols(
        (right * sx).extend(0.0),
        (up * sy).extend(0.0),
        (fwd * sz).extend(0.0),
        pos.extend(1.0),
    )
}

pub fn status_line(view: &ItemView, ui: &ItemUi, prompt: Option<&str>) -> String {
    let sel = view.selected_stack();
    let held = if sel.is_empty() {
        "empty".to_string()
    } else {
        format!("{} x{}", sel.item.def().name, sel.count)
    };
    let recipe = selected_recipe(view)
        .map(|r| format!("[R] {}", r.name))
        .unwrap_or_default();
    let open = if ui.bag_open { "BAG" } else { "hotbar" };
    let p = prompt.unwrap_or("");
    let log = if view.last_log.is_empty() {
        String::new()
    } else {
        format!(" · {}", view.last_log)
    };
    format!("{open}  {held}  {recipe}  {p}{log}")
}
