//! Client-side locomotion.
//!
//! The SpacetimeDB **module** (tables + reducers) lives in
//! `spacetimedb/src/lib.rs`. Do not paste `#[spacetimedb::table]` code here.

use glam::Vec3;

use crate::config::{GROUND_EPS, JUMP_CUT, JUMP_HOLD_GRAVITY, JUMP_MAX_HOLD, JUMP_SPEED};

pub struct PhysicsWorld {
    gravity: Vec3,
    player_pos: Vec3,
    player_vel: Vec3,
    on_ground: bool,
    jump_held: bool,
    jump_time: f32,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            player_pos: Vec3::new(0.0, 0.0, 6.0),
            player_vel: Vec3::ZERO,
            on_ground: true,
            jump_held: false,
            jump_time: 0.0,
        }
    }

    pub fn create_ground(&mut self) {}

    pub fn create_player_capsule(&mut self, pos: Vec3) {
        self.player_pos = pos;
    }

    pub fn add_static_box(&mut self, _pos: Vec3, _half_extents: Vec3) {}

    /// `jump_pressed` is the current Space state (held), not an edge.
    pub fn set_wish_horizontal(&mut self, vx: f32, vz: f32, jump_pressed: bool) {
        self.player_vel.x = vx;
        self.player_vel.z = vz;

        if jump_pressed && self.on_ground {
            self.player_vel.y = JUMP_SPEED;
            self.on_ground = false;
            self.jump_held = true;
            self.jump_time = 0.0;
        } else if !jump_pressed && self.jump_held {
            if self.player_vel.y > 0.0 {
                self.player_vel.y *= JUMP_CUT;
            }
            self.jump_held = false;
        }
    }

    pub fn step(&mut self, dt: f32) {
        let rising = self.player_vel.y > 0.0;
        let hold = self.jump_held && rising && self.jump_time < JUMP_MAX_HOLD;
        if hold {
            self.jump_time += dt;
        }
        let gy = if hold { JUMP_HOLD_GRAVITY } else { self.gravity.y };
        self.player_vel.y += gy * dt;
        self.player_pos += self.player_vel * dt;

        if self.player_pos.y <= 0.0 && self.player_vel.y <= 0.0 {
            self.player_pos.y = 0.0;
            self.player_vel.y = 0.0;
            self.on_ground = true;
            self.jump_held = false;
            self.jump_time = 0.0;
        } else {
            self.on_ground = self.player_pos.y <= GROUND_EPS && self.player_vel.y <= 0.0;
        }
    }

    pub fn player_transform(&self) -> Option<(Vec3, bool)> {
        Some((self.player_pos, self.on_ground))
    }

    pub fn player_velocity(&self) -> Vec3 {
        self.player_vel
    }
}
