use glam::{Mat4, Quat, Vec3};

use crate::items::{ItemId, Stack};
use crate::scene::{DrawRequest, GameObject, Object, ObjectKind, TickCtx};
use crate::vulkan::ModelHandle;

/// World drop. Walk into range to pick up (App auto-pick), or [E] if interactable.
pub struct LootObject {
    pub base: Object,
    pub loot_id: u64,
    pub stack: Stack,
    pub handle: ModelHandle,
    pub bob: f32,
}

impl LootObject {
    pub fn new(mut base: Object, loot_id: u64, stack: Stack, handle: ModelHandle) -> Self {
        base.kind = ObjectKind::Loot;
        base.interactable = true;
        base.interact_radius = crate::items::PICKUP_RANGE;
        base.name = format!("{} x{}", stack.item.def().name, stack.count);
        Self {
            base,
            loot_id,
            stack,
            handle,
            bob: (loot_id as f32) * 0.37,
        }
    }

    fn model_matrix(&self) -> Mat4 {
        let t = self.base.transform.translation + Vec3::Y * (0.22 + self.bob.sin() * 0.08);
        let rot = Quat::from_rotation_y(self.bob * 1.4);
        let scale = if self.stack.item.def().tool {
            Vec3::splat(0.28)
        } else {
            let n = (self.stack.count as f32 / self.stack.item.def().stack.max(1) as f32).clamp(0.25, 1.0);
            Vec3::new(0.18 + 0.10 * n, 0.16 + 0.14 * n, 0.18 + 0.10 * n)
        };
        Mat4::from_scale_rotation_translation(scale, rot, t)
    }
}

impl GameObject for LootObject {
    fn base(&self) -> &Object {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Object {
        &mut self.base
    }

    fn tick(&mut self, ctx: &mut TickCtx) {
        if let Some(l) = ctx.item_view.loot.iter().find(|l| l.id == self.loot_id) {
            self.stack = l.stack;
            self.base.transform.translation = l.pos;
            self.base.name = format!("{} x{}", l.stack.item.def().name, l.stack.count);
        }
        self.bob += ctx.dt * 2.4;
    }

    fn interact(&mut self, ctx: &mut TickCtx) -> bool {
        ctx.items.pickup(self.loot_id)
    }

    fn draws(&self) -> Vec<DrawRequest> {
        if self.stack.item == ItemId::EMPTY {
            return Vec::new();
        }
        vec![DrawRequest {
            handle: self.handle,
            model: self.model_matrix(),
        }]
    }
}
