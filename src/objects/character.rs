use glam::{Mat4, Vec3};

use crate::anim::{skin_primitive, Animator, ClipId};
use crate::assets::{AdventurerClass, RiggedModel, Vertex};
use crate::player::{character_model_matrix, look_forward, look_right};
use crate::scene::{DrawRequest, GameObject, Object, ObjectKind, TickCtx};
use crate::vulkan::ModelHandle;

pub struct AttachedItem {
    pub handle: ModelHandle,
    pub socket: String,
}

/// Skinned adventurer. Embeds [`Object`] and drives KayKit clips from input.
pub struct CharacterObject {
    pub base: Object,
    pub class: AdventurerClass,
    pub rig: RiggedModel,
    pub gpu: ModelHandle,
    pub weapon: Option<AttachedItem>,
    pub animator: Animator,
    pub yaw: f32,
    pub sitting: bool,
    pub attacking: bool,
    pub interacting: bool,
    pub was_grounded: bool,
    pub oneshot_done: bool,
    pub skinned: Vec<Vec<Vertex>>,
    pub posed_locals: Vec<Mat4>,
    pub is_local: bool,
}

impl CharacterObject {
    pub fn new(
        mut base: Object,
        class: AdventurerClass,
        rig: RiggedModel,
        gpu: ModelHandle,
        weapon: Option<AttachedItem>,
        yaw: f32,
        is_local: bool,
    ) -> Self {
        base.kind = ObjectKind::Character;
        let posed_locals = rig.skeleton.rest_locals();
        Self {
            base,
            class,
            rig,
            gpu,
            weapon,
            animator: Animator::default(),
            yaw,
            sitting: false,
            attacking: false,
            interacting: false,
            was_grounded: true,
            oneshot_done: true,
            skinned: Vec::new(),
            posed_locals,
            is_local,
        }
    }

    pub fn model_matrix(&self) -> Mat4 {
        character_model_matrix(self.base.transform.translation, self.yaw)
    }

    pub fn socket_world(&self, name: &str) -> Option<Mat4> {
        self.rig
            .skeleton
            .socket(&self.posed_locals, name)
            .map(|local| self.model_matrix() * local)
    }

    fn choose_clip(&mut self, ctx: &TickCtx) {
        if self.is_local && ctx.sit_toggle {
            self.sitting = !self.sitting;
            if self.sitting {
                self.attacking = false;
                self.interacting = false;
            }
        }

        if self.attacking && self.oneshot_done {
            self.attacking = false;
        }
        if self.interacting && self.oneshot_done {
            self.interacting = false;
        }

        if self.is_local && ctx.attack && !self.sitting && !self.attacking {
            self.attacking = true;
            self.oneshot_done = false;
            self.animator.play(ClipId::Attack.key(), false, 1.0, 0.08);
            return;
        }
        if self.is_local && ctx.interact && !self.sitting && !self.interacting {
            self.interacting = true;
            self.oneshot_done = false;
            self.animator.play(ClipId::Interact.key(), false, 1.0, 0.08);
            return;
        }

        if self.attacking || self.interacting {
            return;
        }

        if self.sitting {
            self.animator.play(ClipId::Idle.key(), true, 1.0, 0.2);
            return;
        }

        let grounded = if self.is_local { ctx.grounded } else { true };
        let landed = grounded && !self.was_grounded;
        let left_ground = !grounded && self.was_grounded;
        self.was_grounded = grounded;

        if left_ground {
            self.animator.play(ClipId::JumpStart.key(), false, 1.0, 0.06);
            return;
        }
        if landed {
            self.animator.play(ClipId::JumpLand.key(), false, 1.0, 0.05);
            return;
        }
        if !grounded {
            if self.animator.current_name() != Some(ClipId::JumpStart.key()) || self.oneshot_done {
                self.animator.play(ClipId::JumpAir.key(), true, 1.0, 0.1);
            }
            return;
        }

        if self.animator.current_name() == Some(ClipId::JumpLand.key()) && !self.oneshot_done {
            return;
        }

        if !self.is_local {
            self.animator.play(ClipId::Idle.key(), true, 1.0, 0.2);
            return;
        }

        let horiz = Vec3::new(ctx.moving.x, 0.0, ctx.moving.z);
        let speed = horiz.length();
        if speed < 0.15 {
            self.animator.play(ClipId::Idle.key(), true, 1.0, 0.15);
            return;
        }

        let dir = horiz.normalize_or_zero();
        let along = dir.dot(look_forward(self.yaw));
        let side = dir.dot(look_right(self.yaw));

        if ctx.sprinting && along > 0.35 {
            self.animator.play(ClipId::Run.key(), true, 1.0, 0.12);
        } else if along < -0.35 {
            self.animator.play(ClipId::Walk.key(), true, -1.0, 0.12);
        } else if side.abs() > along.abs() {
            let spd = if side < 0.0 { 0.9 } else { 1.05 };
            self.animator.play(ClipId::Walk.key(), true, spd, 0.12);
        } else {
            self.animator.play(ClipId::Walk.key(), true, 1.0, 0.12);
        }
    }

    fn apply_pose(&mut self, dt: f32, ctx: &TickCtx) {
        let target_sit = if self.sitting { 1.0 } else { 0.0 };
        self.animator.sit_amount +=
            (target_sit - self.animator.sit_amount) * (1.0 - (-6.0 * dt).exp());
        self.animator.tick(dt, ctx.library);
        self.oneshot_done = self.animator.finished(ctx.library);
        self.posed_locals = self.animator.sample(ctx.library, &self.rig.skeleton);
        let palette = self.rig.skeleton.palette(&self.posed_locals);
        self.skinned = self
            .rig
            .primitives
            .iter()
            .map(|p| skin_primitive(p, &palette))
            .collect();
    }
}

impl GameObject for CharacterObject {
    fn base(&self) -> &Object {
        &self.base
    }
    fn base_mut(&mut self) -> &mut Object {
        &mut self.base
    }

    fn tick(&mut self, ctx: &mut TickCtx) {
        if self.is_local {
            self.yaw = ctx.player_yaw;
            self.base.transform.translation = ctx.player_pos;
        }
        self.choose_clip(ctx);
        self.apply_pose(ctx.dt, ctx);
    }

    fn sync_local(&mut self, pos: Vec3, yaw: f32) {
        if self.is_local {
            self.base.transform.translation = pos;
            self.yaw = yaw;
        }
    }

    fn skinned_upload(&self) -> Option<(ModelHandle, &[Vec<Vertex>])> {
        if self.skinned.is_empty() {
            None
        } else {
            Some((self.gpu, &self.skinned))
        }
    }

    fn draws(&self) -> Vec<DrawRequest> {
        let model = self.model_matrix();
        let mut out = vec![DrawRequest {
            handle: self.gpu,
            model,
        }];
        if let Some(w) = &self.weapon {
            let socket = self.socket_world(&w.socket).unwrap_or(model);
            out.push(DrawRequest {
                handle: w.handle,
                model: socket,
            });
        }
        out
    }
}
