//! Client-side locomotion and collision.
//!
//! Swept AABB against voxel cells, static boxes (stations / crates / placed
//! builds / loot) and other actors (NPCs / remote players). The y=0 test
//! plane is gone — the floor is the terrain itself.
//!
//! The SpacetimeDB **module** (tables + reducers) lives in
//! `spacetimedb/src/lib.rs`. Do not paste `#[spacetimedb::table]` code here.

use glam::Vec3;

use crate::config::{
    GROUND_EPS, JUMP_CUT, JUMP_HOLD_GRAVITY, JUMP_MAX_HOLD, JUMP_SPEED, PLAYER_HEIGHT,
    PLAYER_RADIUS,
};

/// Axis-aligned box in world metres. `min` is inclusive, `max` exclusive
/// in the sense of "surface you stand on is `max.y`".
///
/// **Takes:** two corners from whoever built the collider (voxels, stations,
/// actors). **Gives:** overlap / snap queries to the resolver.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    /// **Takes:** centre and half-extents (the Rapier-style box the rest of
    /// the game already uses). **Gives:** min/max AABB.
    pub fn from_center_half(center: Vec3, half: Vec3) -> Self {
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Player / NPC capsule as a box.
    ///
    /// **Takes:** feet position (`Player.position`), radius, height.
    /// **Gives:** an AABB whose bottom is the feet.
    pub fn from_feet(feet: Vec3, radius: f32, height: f32) -> Self {
        Self {
            min: Vec3::new(feet.x - radius, feet.y, feet.z - radius),
            max: Vec3::new(feet.x + radius, feet.y + height, feet.z + radius),
        }
    }

    pub fn overlaps(self, o: Aabb) -> bool {
        self.min.x < o.max.x
            && self.max.x > o.min.x
            && self.min.y < o.max.y
            && self.max.y > o.min.y
            && self.min.z < o.max.z
            && self.max.z > o.min.z
    }

    fn translate(self, d: Vec3) -> Self {
        Self {
            min: self.min + d,
            max: self.max + d,
        }
    }
}

pub struct PhysicsWorld {
    gravity: Vec3,
    player_pos: Vec3,
    player_vel: Vec3,
    on_ground: bool,
    jump_held: bool,
    jump_time: f32,
    /// Stations, crates, placed builds, world loot. Rebuilt each tick by
    /// [`App::rebuild_colliders`].
    statics: Vec<Aabb>,
    /// NPCs and remote players. Rebuilt each tick; the local capsule is
    /// *not* in this list.
    actors: Vec<Aabb>,
}

impl PhysicsWorld {
    /// **Takes:** world gravity from [`crate::config::GRAVITY`].
    /// **Gives:** a world with an empty collider list and a default spawn
    /// that [`create_player_capsule`] overwrites once the terrain height is
    /// known.
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            player_pos: Vec3::new(0.0, 8.0, 6.0),
            player_vel: Vec3::ZERO,
            on_ground: false,
            jump_held: false,
            jump_time: 0.0,
            statics: Vec::new(),
            actors: Vec::new(),
        }
    }

    /// Kept so old call sites compile. The infinite test plane is gone —
    /// occupancy comes from [`crate::voxel::solid_at`].
    #[allow(dead_code)]
    pub fn create_ground(&mut self) {}

    /// **Takes:** feet position from [`crate::player::Player`] (already lifted
    /// onto [`crate::voxel::stand_y`]).
    /// **Gives:** the kinematic capsule origin this world integrates.
    pub fn create_player_capsule(&mut self, pos: Vec3) {
        self.player_pos = pos;
        self.player_vel = Vec3::ZERO;
        self.on_ground = false;
    }

    /// Drop every dynamic collider. Called at the start of each locomotion
    /// frame before the scene re-feeds boxes.
    ///
    /// **Takes:** nothing. **Gives:** empty `statics` / `actors`.
    /// **Source:** [`crate::app::App::drive_player`].
    pub fn clear_colliders(&mut self) {
        self.statics.clear();
        self.actors.clear();
    }

    /// Register a solid box the player cannot walk through.
    ///
    /// **Takes:** world-space centre + half-extents from stations
    /// (`StationKind::half_extents`), crates (`LocalWorld` entities), placed
    /// [`crate::rpg::BuildPiece`]s, and loot gems.
    /// **Gives:** an AABB pushed onto `statics`, consumed by [`Self::step`].
    pub fn add_static_box(&mut self, pos: Vec3, half_extents: Vec3) {
        self.statics
            .push(Aabb::from_center_half(pos, half_extents));
    }

    /// Register another character (NPC or remote player) as a blocking capsule.
    ///
    /// **Takes:** their feet, radius, height (same scale as the local player).
    /// **Gives:** an AABB on `actors`. **Source:** scene `ObjectKind::Character`
    /// nodes and `GameMode::Multiplayer.remote_players`.
    pub fn add_actor_capsule(&mut self, feet: Vec3, radius: f32, height: f32) {
        self.actors.push(Aabb::from_feet(feet, radius, height));
    }

    /// Copy wish XZ from the input step and apply jump if grounded.
    ///
    /// **Takes:** `vx`/`vz` from [`crate::player::Player::update_movement`]
    /// (already scaled by hero DEX) and the current Space key.
    /// **Gives:** updated `player_vel` that [`Self::step`] integrates.
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

    /// Integrate one frame against voxels + statics + actors.
    ///
    /// **Takes:** `dt` from the render/locomotion clock, and `solid(wx,wy,wz)`
    /// from [`crate::voxel::solid_at`] (loaded chunks + heightfield fallback).
    /// **Gives:** new `player_pos` / `player_vel` / `on_ground`.
    /// **Goes to:** [`crate::app::App::drive_player`] which copies them onto
    /// [`crate::player::Player`].
    pub fn step(&mut self, dt: f32, solid: impl Fn(i32, i32, i32) -> bool) {
        let rising = self.player_vel.y > 0.0;
        let hold = self.jump_held && rising && self.jump_time < JUMP_MAX_HOLD;
        if hold {
            self.jump_time += dt;
        }
        let gy = if hold { JUMP_HOLD_GRAVITY } else { self.gravity.y };
        self.player_vel.y += gy * dt;

        let mut boxes = collect_voxels(self.player_pos, self.player_vel, dt, &solid);
        boxes.extend_from_slice(&self.statics);
        boxes.extend_from_slice(&self.actors);

        let mut grounded = false;
        // Y first so we land on floors before sliding along walls.
        let dy = self.player_vel.y * dt;
        let (ny, hit_floor, hit_ceil) = resolve_axis(self.player_pos, PLAYER_RADIUS, PLAYER_HEIGHT, 1, dy, &boxes);
        self.player_pos.y = ny;
        if hit_floor || hit_ceil {
            self.player_vel.y = 0.0;
        }
        if hit_floor {
            grounded = true;
            self.jump_held = false;
            self.jump_time = 0.0;
        }

        let dx = self.player_vel.x * dt;
        let (nx, hit_x, _) = resolve_axis(self.player_pos, PLAYER_RADIUS, PLAYER_HEIGHT, 0, dx, &boxes);
        self.player_pos.x = nx;
        if hit_x {
            self.player_vel.x = 0.0;
        }

        let dz = self.player_vel.z * dt;
        let (nz, hit_z, _) = resolve_axis(self.player_pos, PLAYER_RADIUS, PLAYER_HEIGHT, 2, dz, &boxes);
        self.player_pos.z = nz;
        if hit_z {
            self.player_vel.z = 0.0;
        }

        // Tiny downward probe so walking off a ledge isn't delayed a frame,
        // and so we stay "grounded" on flat terrain without sinking.
        if !grounded && self.player_vel.y <= 0.0 {
            let probe = resolve_axis(
                self.player_pos,
                PLAYER_RADIUS,
                PLAYER_HEIGHT,
                1,
                -GROUND_EPS * 2.0,
                &boxes,
            );
            if probe.1 {
                self.player_pos.y = probe.0;
                grounded = true;
                self.player_vel.y = 0.0;
            }
        }
        self.on_ground = grounded;

        // Safety net if something punches through the heightfield.
        if self.player_pos.y < -8.0 {
            self.player_pos.y = 16.0;
            self.player_vel.y = 0.0;
        }
    }

    pub fn player_transform(&self) -> Option<(Vec3, bool)> {
        Some((self.player_pos, self.on_ground))
    }

    pub fn player_velocity(&self) -> Vec3 {
        self.player_vel
    }
}

/// Gather unit cubes for every solid voxel the swept capsule might touch.
///
/// **Takes:** current feet, velocity, `dt`, and the occupancy closure.
/// **Gives:** AABBs in world metres (`[i, i+1]` per axis).
fn collect_voxels(
    feet: Vec3,
    vel: Vec3,
    dt: f32,
    solid: &impl Fn(i32, i32, i32) -> bool,
) -> Vec<Aabb> {
    let pad = Vec3::new(PLAYER_RADIUS + 0.05, 0.05, PLAYER_RADIUS + 0.05);
    let body = Aabb::from_feet(feet, PLAYER_RADIUS, PLAYER_HEIGHT);
    let sweep = vel * dt;
    let min = (body.min + sweep.min(Vec3::ZERO)) - pad;
    let max = (body.max + sweep.max(Vec3::ZERO)) + pad;
    let x0 = min.x.floor() as i32;
    let y0 = min.y.floor() as i32;
    let z0 = min.z.floor() as i32;
    let x1 = max.x.floor() as i32;
    let y1 = max.y.floor() as i32;
    let z1 = max.z.floor() as i32;
    let mut out = Vec::new();
    for y in y0..=y1 {
        for z in z0..=z1 {
            for x in x0..=x1 {
                if solid(x, y, z) {
                    out.push(Aabb {
                        min: Vec3::new(x as f32, y as f32, z as f32),
                        max: Vec3::new(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0),
                    });
                }
            }
        }
    }
    out
}

/// Move the capsule along one axis and snap out of any overlapping box.
///
/// **Takes:** feet, capsule size, axis (`0=X 1=Y 2=Z`), proposed delta, solids.
/// **Gives:** `(new_component, hit_negative, hit_positive)` — for Y that is
/// `(new_y, hit_floor, hit_ceiling)`.
fn resolve_axis(
    feet: Vec3,
    radius: f32,
    height: f32,
    axis: usize,
    delta: f32,
    boxes: &[Aabb],
) -> (f32, bool, bool) {
    if delta == 0.0 {
        return (feet[axis], false, false);
    }
    let mut moved = Aabb::from_feet(feet, radius, height);
    let mut off = Vec3::ZERO;
    off[axis] = delta;
    moved = moved.translate(off);

    let mut hit_neg = false;
    let mut hit_pos = false;
    for b in boxes {
        if !moved.overlaps(*b) {
            continue;
        }
        if delta > 0.0 {
            // Hitting the min face of the obstacle.
            let depth = moved.max[axis] - b.min[axis];
            if depth > 0.0 && depth < (if axis == 1 { height } else { radius * 2.0 }) + delta.abs() + 0.01
            {
                moved.max[axis] = b.min[axis];
                moved.min[axis] = if axis == 1 {
                    moved.max[axis] - height
                } else {
                    moved.max[axis] - radius * 2.0
                };
                hit_pos = true;
            }
        } else {
            let depth = b.max[axis] - moved.min[axis];
            if depth > 0.0 && depth < (if axis == 1 { height } else { radius * 2.0 }) + delta.abs() + 0.01
            {
                moved.min[axis] = b.max[axis];
                moved.max[axis] = if axis == 1 {
                    moved.min[axis] + height
                } else {
                    moved.min[axis] + radius * 2.0
                };
                hit_neg = true;
            }
        }
    }
    let out = if axis == 1 {
        moved.min.y
    } else if axis == 0 {
        (moved.min.x + moved.max.x) * 0.5
    } else {
        (moved.min.z + moved.max.z) * 0.5
    };
    (out, hit_neg, hit_pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_ground(x: i32, y: i32, z: i32) -> bool {
        let _ = (x, z);
        y <= 4
    }

    #[test]
    fn lands_on_voxel_surface_not_y_zero() {
        let mut w = PhysicsWorld::new(Vec3::new(0.0, -22.0, 0.0));
        w.create_player_capsule(Vec3::new(0.5, 12.0, 0.5));
        for _ in 0..90 {
            w.set_wish_horizontal(0.0, 0.0, false);
            w.step(1.0 / 60.0, flat_ground);
        }
        let (pos, grounded) = w.player_transform().unwrap();
        assert!(grounded, "should have landed");
        assert!(
            (pos.y - 5.0).abs() < 0.05,
            "feet should sit on top of y=4 block, got {}",
            pos.y
        );
    }

    #[test]
    fn wall_blocks_horizontal() {
        let mut w = PhysicsWorld::new(Vec3::ZERO);
        w.create_player_capsule(Vec3::new(0.5, 5.0, 0.5));
        w.add_static_box(Vec3::new(2.0, 5.8, 0.5), Vec3::new(0.5, 1.0, 0.5));
        w.set_wish_horizontal(8.0, 0.0, false);
        for _ in 0..30 {
            w.step(1.0 / 60.0, flat_ground);
            w.set_wish_horizontal(8.0, 0.0, false);
        }
        let (pos, _) = w.player_transform().unwrap();
        assert!(pos.x < 1.4, "should have stopped at the wall, x={}", pos.x);
    }

    #[test]
    fn actor_capsule_blocks_player() {
        let mut w = PhysicsWorld::new(Vec3::ZERO);
        w.create_player_capsule(Vec3::new(0.5, 5.0, 0.5));
        w.add_actor_capsule(Vec3::new(2.0, 5.0, 0.5), PLAYER_RADIUS, PLAYER_HEIGHT);
        w.set_wish_horizontal(8.0, 0.0, false);
        for _ in 0..30 {
            w.step(1.0 / 60.0, flat_ground);
            w.set_wish_horizontal(8.0, 0.0, false);
        }
        let (pos, _) = w.player_transform().unwrap();
        assert!(pos.x < 1.6, "should have stopped at the NPC, x={}", pos.x);
    }
}