//! Modular world station (chest / furnace / workbench).
//!
//! Add a new kind by:
//! 1. `StationKind` + slot count / craft mapping in `spacetime_items`
//! 2. Mesh factory in `assets/primitives.rs`
//! 3. Optional visual in [`StationObject::tick`]

use glam::{Mat4, Quat, Vec3};

use crate::items::{StationKind, STATION_RANGE};
use crate::scene::{DrawRequest, GameObject, Object, ObjectKind, TickCtx};
use crate::vulkan::ModelHandle;

#[derive(Clone, Copy, Debug)]
pub struct StationMeshes {
    pub body: ModelHandle,
    pub lid: Option<ModelHandle>,
    pub ember: Option<ModelHandle>,
}

pub struct StationObject {
    pub base: Object,
    pub station_id: u64,
    pub kind: StationKind,
    pub meshes: StationMeshes,
    pub open: bool,
    pub lid_angle: f32,
    pub glow: f32,
}

impl StationObject {
    pub fn new(
        mut base: Object,
        station_id: u64,
        kind: StationKind,
        meshes: StationMeshes,
    ) -> Self {
        base.kind = ObjectKind::Station;
        base.interactable = true;
        base.interact_radius = STATION_RANGE;
        base.name = kind.name().to_string();
        Self {
            base,
            station_id,
            kind,
            meshes,
            open: false,
            lid_angle: 0.0,
            glow: 0.0,
        }
    }

    fn lid_matrix(&self) -> Mat4 {
        let world = self.base.transform.matrix();
        let hinge = Vec3::new(0.0, 0.45, -0.26);
        world
            * Mat4::from_translation(hinge)
            * Mat4::from_quat(Quat::from_rotation_x(-self.lid_angle))
            * Mat4::from_translation(-hinge)
    }

    fn ember_matrix(&self) -> Mat4 {
        let world = self.base.transform.matrix();
        let s = 0.25 + 0.35 * self.glow;
        world
            * Mat4::from_translation(Vec3::new(0.0, 0.18, 0.12))
            * Mat4::from_scale(Vec3::new(s, s * 0.6, s))
    }
}

impl GameObject for StationObject {
    fn base(&self) -> &Object {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Object {
        &mut self.base
    }

    fn tick(&mut self, ctx: &mut TickCtx) {
        if let Some(st) = ctx.item_view.stations.iter().find(|s| s.id == self.station_id) {
            self.base.transform.translation = st.pos;
            self.kind = st.kind;
            let lit = (st.fuel as f32 / 160.0).clamp(0.0, 1.0);
            let k = 1.0 - (-6.0 * ctx.dt).exp();
            self.glow += (lit - self.glow) * k;
        }
        self.open = ctx.item_view.open_station == Some(self.station_id);
        let target = if self.open && self.kind == StationKind::Chest {
            1.85
        } else {
            0.0
        };
        let k = 1.0 - (-8.0 * ctx.dt).exp();
        self.lid_angle += (target - self.lid_angle) * k;
    }

    fn interact(&mut self, ctx: &mut TickCtx) -> bool {
        ctx.items.toggle_station(self.station_id);
        true
    }

    fn draws(&self) -> Vec<DrawRequest> {
        let world = self.base.transform.matrix();
        let mut out = vec![DrawRequest {
            handle: self.meshes.body,
            model: world,
        }];
        if let Some(lid) = self.meshes.lid {
            out.push(DrawRequest {
                handle: lid,
                model: self.lid_matrix(),
            });
        }
        if let Some(ember) = self.meshes.ember {
            if self.glow > 0.04 {
                out.push(DrawRequest {
                    handle: ember,
                    model: self.ember_matrix(),
                });
            }
        }
        out
    }
}
