use glam::{Mat4, Vec3};
use crate::config::{JUMP_FORCE, MOVE_SPEED, SPRINT_MULTIPLIER};
use crate::input::InputState;

pub struct Player {
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

impl Player {
    pub fn new(spawn: Vec3) -> Self {
        Self {
            position: spawn,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
        }
    }

    pub fn update_movement(&mut self, input: &InputState, dt: f32) {
        let forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(forward.z, 0.0, -forward.x);

        let mut wish = Vec3::ZERO;
        if input.forward { wish += forward; }
        if input.back    { wish -= forward; }
        if input.left    { wish -= right; }
        if input.right   { wish += right; }

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

    pub fn view_matrix(&self) -> Mat4 {
        let eye = self.eye_position();
        let direction = Vec3::new(
            -self.yaw.sin() * self.pitch.cos(),
             self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        );
        Mat4::look_to_rh(eye, direction, Vec3::Y)
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