use glam::Vec3;

use crate::assets::AdventurerClass;

pub const WINDOW_TITLE: &str = "Spacetime — KayKit Adventurers";
pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 720;

pub const SPACETIME_URI: &str = "ws://localhost:3000";
pub const SPACETIME_DB_NAME: &str = "game";

pub const MOVE_SPEED: f32 = 6.0;
pub const SPRINT_MULTIPLIER: f32 = 1.6;

/// Initial upward speed when Space is pressed on the ground.
pub const JUMP_SPEED: f32 = 7.2;
/// Gravity while Space is still held and the player is rising (weaker = higher jump).
pub const JUMP_HOLD_GRAVITY: f32 = -11.0;
/// Max time the hold-gravity bonus applies.
pub const JUMP_MAX_HOLD: f32 = 0.22;
/// Multiply upward velocity when Space is released mid-jump (short tap = short hop).
pub const JUMP_CUT: f32 = 0.42;
pub const JUMP_FORCE: f32 = JUMP_SPEED;

pub const MOUSE_SENSITIVITY: f32 = 0.002;
pub const GRAVITY: Vec3 = Vec3::new(0.0, -22.0, 0.0);
pub const GROUND_EPS: f32 = 0.02;

/// Horizontal radius of the player collision capsule (XZ).
///
/// **Takes:** nothing (constant). **Gives:** half-width for [`crate::physics`].
/// **Source:** tuned to the KayKit adventurer footprint.
/// **Goes to:** [`crate::physics::PhysicsWorld`] AABB and actor capsules.
pub const PLAYER_RADIUS: f32 = 0.35;
/// Player collision height from feet (`Player.position.y`) to the top of the head.
///
/// **Takes:** nothing. **Gives:** vertical extent for swept AABB.
/// **Source:** KayKit adult-human scale (~1.7 m).
/// **Goes to:** [`crate::physics::PhysicsWorld::step`].
pub const PLAYER_HEIGHT: f32 = 1.72;

/// Pitch stop just shy of ±90° so `look_at_rh` never gets a look vector
/// parallel to world +Y (that would zero the camera basis).
///
/// **Takes:** nothing. **Gives:** clamp used by [`crate::player::Player::apply_look`].
/// **Source:** `π/2 − 1°`. **Goes to:** chase-camera `look_dir`.
pub const PITCH_LIMIT: f32 = std::f32::consts::FRAC_PI_2 - 0.018;

pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const MAX_FRAME_TIME: f32 = 0.05;

/// Default hero loaded from the KayKit pack.
pub const LOCAL_CLASS: AdventurerClass = AdventurerClass::Knight;
