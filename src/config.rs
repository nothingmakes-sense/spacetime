use glam::Vec3;

use crate::assets::AdventurerClass;

pub const WINDOW_TITLE: &str = "Spacetime — KayKit Adventurers";
pub const WINDOW_WIDTH: u32 = 1280;
pub const WINDOW_HEIGHT: u32 = 720;

pub const SPACETIME_URI: &str = "ws://localhost:3000";
pub const SPACETIME_DB_NAME: &str = "game";

pub const MOVE_SPEED: f32 = 6.0;
pub const SPRINT_MULTIPLIER: f32 = 1.6;
pub const JUMP_FORCE: f32 = 8.0;
pub const MOUSE_SENSITIVITY: f32 = 0.002;
pub const GRAVITY: Vec3 = Vec3::new(0.0, -18.0, 0.0);

pub const FIXED_DT: f32 = 1.0 / 60.0;
pub const MAX_FRAME_TIME: f32 = 0.05;

/// Default hero loaded from the KayKit pack.
pub const LOCAL_CLASS: AdventurerClass = AdventurerClass::Knight;
