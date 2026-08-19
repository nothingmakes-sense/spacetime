use glam::{Mat4, Quat, Vec3};

use crate::config::{MOVE_SPEED, PITCH_LIMIT, SPRINT_MULTIPLIER};
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

/// Aim vector from yaw + pitch. Negative pitch looks down, +π/2 looks straight up.
///
/// **Takes:** heading (`yaw`) and elevation (`pitch`) from [`Player`].
/// **Gives:** a unit world vector the chase camera and build-place ray share.
/// **Source:** [`Player::apply_look`] (mouse deltas × sensitivity).
/// **Goes to:** [`Player::chase_view_at`] and world RMB place.
pub fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
    let cp = pitch.cos();
    Vec3::new(-yaw.sin() * cp, pitch.sin(), -yaw.cos() * cp)
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

    /// Apply a mouse delta to heading / elevation.
    ///
    /// **Takes:** raw device `dx`/`dy` and the sensitivity from
    /// [`crate::settings::Settings::mouse_sensitivity`].
    /// **Gives:** updated `yaw` (unbounded) and `pitch` clamped to
    /// [`crate::config::PITCH_LIMIT`] so the player can look straight up
    /// or straight down without flipping the camera.
    /// **Goes to:** [`look_dir`] / [`Self::chase_view_at`].
    pub fn apply_look(&mut self, dx: f64, dy: f64, sensitivity: f32) {
        // Mouse right increases world-right heading (yaw decreases → toward +X at yaw 0).
        self.yaw -= dx as f32 * sensitivity;
        self.pitch -= dy as f32 * sensitivity;
        self.pitch = self.pitch.clamp(-PITCH_LIMIT, PITCH_LIMIT);
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
        // Vertical impulse lives in PhysicsWorld so hold-to-jump can cut/extend.
    }

    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, 1.6, 0.0)
    }

    /// Third-person orbit camera.
    ///
    /// **Takes:** the render-interpolated feet `pos` (so the camera does not
    /// quantize to the 60 Hz physics step) plus this player's yaw/pitch.
    /// **Gives:** `(view_matrix, eye_world)` for the Vulkan camera UBO.
    /// **Source:** [`Self::apply_look`] for orientation, [`crate::physics`]
    /// for `pos`. **Goes to:** [`crate::app::App::render`].
    pub fn chase_view_at(&self, pos: Vec3) -> (Mat4, Vec3) {
        let target = pos + Vec3::Y * 1.4;
        let dir = look_dir(self.yaw, self.pitch);
        let eye = target - dir * 5.0;
        (Mat4::look_at_rh(eye, target, Vec3::Y), eye)
    }

    pub fn chase_view_matrix(&self) -> (Mat4, Vec3) {
        self.chase_view_at(self.position)
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

    #[test]
    fn pitch_reaches_straight_up_and_down() {
        let mut p = Player::new(Vec3::ZERO);
        p.apply_look(0.0, 10_000.0, 1.0);
        assert!((p.pitch + PITCH_LIMIT).abs() < 1e-4, "down pitch={}", p.pitch);
        assert!(look_dir(p.yaw, p.pitch).y < -0.99);

        p.apply_look(0.0, -20_000.0, 1.0);
        assert!((p.pitch - PITCH_LIMIT).abs() < 1e-4, "up pitch={}", p.pitch);
        assert!(look_dir(p.yaw, p.pitch).y > 0.99);
    }
}