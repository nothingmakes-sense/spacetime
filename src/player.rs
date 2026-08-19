use glam::{Mat4, Quat, Vec3};

use crate::config::{JUMP_FORCE, MOUSE_SENSITIVITY, MOVE_SPEED, SPRINT_MULTIPLIER};
use crate::input::InputState;

/// KayKit Adventurers face +Z in rest pose (cape on −Z, visor on +Z).
/// Game yaw 0 looks down −Z with the chase camera on +Z, so meshes need a
/// half-turn or they stare into the lens.
const MESH_YAW_OFFSET: f32 = std::f32::consts::PI;

/// Walk / look direction on XZ. Yaw 0 = −Z, increasing yaw turns left (CCW).
pub fn look_forward(yaw: f32) -> Vec3 {
    Vec3::new(-yaw.sin(), 0.0, -yaw.cos())
}

/// Screen-right on XZ: `forward × +Y` in a right-handed Y-up frame.
/// Yaw 0 ⇒ +X, so D strafes right of the camera, not left.
pub fn look_right(yaw: f32) -> Vec3 {
    look_forward(yaw).cross(Vec3::Y)
}

/// Model matrix that faces `look_forward(yaw)`.
pub fn character_model_matrix(position: Vec3, yaw: f32) -> Mat4 {
    Mat4::from_rotation_translation(Quat::from_rotation_y(yaw + MESH_YAW_OFFSET), position)
}

pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
    pub sitting: bool,
}

impl Player {
    pub fn new(spawn: Vec3) -> Self {
        Self {
            position: spawn,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: -0.25,
            on_ground: false,
            sitting: false,
        }
    }

    pub fn apply_look(&mut self, dx: f64, dy: f64) {
        // Mouse right increases world-right heading (yaw decreases → toward +X at yaw 0).
        self.yaw -= dx as f32 * MOUSE_SENSITIVITY;
        self.pitch -= dy as f32 * MOUSE_SENSITIVITY;
        self.pitch = self.pitch.clamp(-1.35, 0.35);
    }

    pub fn forward_xz(&self) -> Vec3 {
        look_forward(self.yaw)
    }

    pub fn update_movement(&mut self, input: &InputState, _dt: f32) {
        if self.sitting {
            self.velocity.x = 0.0;
            self.velocity.z = 0.0;
            return;
        }
        let forward = look_forward(self.yaw);
        let right = look_right(self.yaw);

        let mut wish = Vec3::ZERO;
        if input.forward {
            wish += forward;
        }
        if input.back {
            wish -= forward;
        }
        if input.left {
            wish -= right;
        }
        if input.right {
            wish += right;
        }

        if wish.length_squared() > 0.0 {
            wish = wish.normalize();
        }

        let speed = if input.sprint {
            MOVE_SPEED * SPRINT_MULTIPLIER
        } else {
            MOVE_SPEED
        };

        self.velocity.x = wish.x * speed;
        self.velocity.z = wish.z * speed;

        if input.jump && self.on_ground {
            self.velocity.y = JUMP_FORCE;
            self.on_ground = false;
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, 1.6, 0.0)
    }

    /// Third-person chase camera, behind the mesh, looking at the torso.
    pub fn chase_view_matrix(&self) -> (Mat4, Vec3) {
        let forward = self.forward_xz();
        let eye = self.position + Vec3::Y * 2.4 - forward * 5.0 + Vec3::Y * (-self.pitch * 1.2);
        let target = self.position + Vec3::Y * 1.15;
        (Mat4::look_at_rh(eye, target, Vec3::Y), eye)
    }

    pub fn model_matrix(&self) -> Mat4 {
        character_model_matrix(self.position, self.yaw)
    }

    pub fn apply_simple_physics(&mut self, gravity: Vec3, dt: f32) {
        self.velocity += gravity * dt;
        self.position += self.velocity * dt;

        if self.position.y < 0.0 {
            self.position.y = 0.0;
            self.velocity.y = 0.0;
            self.on_ground = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_zero_faces_neg_z_and_right_is_pos_x() {
        let f = look_forward(0.0);
        let r = look_right(0.0);
        assert!(f.z < -0.99 && f.x.abs() < 1e-5, "forward={f}");
        assert!(r.x > 0.99 && r.z.abs() < 1e-5, "right={r}");
    }

    #[test]
    fn mesh_faces_walk_direction() {
        // Rest-pose +Z, rotated by π, must land on −Z (walk forward at yaw 0).
        let rotated = Quat::from_rotation_y(MESH_YAW_OFFSET) * Vec3::Z;
        assert!(rotated.z < -0.99 && rotated.x.abs() < 1e-5, "rotated={rotated}");
    }
}
