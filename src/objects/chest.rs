use glam::{Mat4, Quat, Vec3};

use crate::scene::{DrawRequest, GameObject, Object, ObjectKind, TickCtx};
use crate::vulkan::ModelHandle;

/// Interactable chest: lid hinges open on [E]. Demonstrates a custom asset
/// built on the shared [`Object`] toolkit (no unique GLB required).
pub struct ChestObject {
    pub base: Object,
    pub body: ModelHandle,
    pub lid: ModelHandle,
    pub open: bool,
    pub lid_angle: f32,
}

impl ChestObject {
    pub fn new(mut base: Object, body: ModelHandle, lid: ModelHandle) -> Self {
        base.kind = ObjectKind::Chest;
        base.interactable = true;
        base.interact_radius = 2.4;
        Self {
            base,
            body,
            lid,
            open: false,
            lid_angle: 0.0,
        }
    }

    fn lid_matrix(&self) -> Mat4 {
        let world = self.base.transform.matrix();
        // Hinge along the back top edge of the 0.8×0.45×0.55 body.
        let hinge = Vec3::new(0.0, 0.45, -0.26);
        world
            * Mat4::from_translation(hinge)
            * Mat4::from_quat(Quat::from_rotation_x(-self.lid_angle))
            * Mat4::from_translation(-hinge)
    }
}

impl GameObject for ChestObject {
    fn base(&self) -> &Object {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Object {
        &mut self.base
    }

    fn tick(&mut self, ctx: &mut TickCtx) {
        let target = if self.open { 1.85 } else { 0.0 };
        let k = 1.0 - (-8.0 * ctx.dt).exp();
        self.lid_angle += (target - self.lid_angle) * k;
    }

    fn interact(&mut self, _ctx: &mut TickCtx) -> bool {
        self.open = !self.open;
        log::info!(
            "chest '{}' {}",
            self.base.name,
            if self.open { "opened" } else { "closed" }
        );
        true
    }

    fn draws(&self) -> Vec<DrawRequest> {
        let world = self.base.transform.matrix();
        vec![
            DrawRequest {
                handle: self.body,
                model: world,
            },
            DrawRequest {
                handle: self.lid,
                model: self.lid_matrix(),
            },
        ]
    }
}
