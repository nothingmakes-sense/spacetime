use glam::Vec3;

pub struct PhysicsWorld {
    gravity: Vec3,
    player_pos: Vec3,
    player_vel: Vec3,
    on_ground: bool,
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            player_pos: Vec3::new(0.0, 2.0, 5.0),
            player_vel: Vec3::ZERO,
            on_ground: false,
        }
    }

    pub fn create_ground(&mut self) {}
    pub fn create_player_capsule(&mut self, pos: Vec3) {
        self.player_pos = pos;
    }
    pub fn add_static_box(&mut self, _pos: Vec3, _half_extents: Vec3) {}

    pub fn step(&mut self, dt: f32) {
        self.player_vel += self.gravity * dt;
        self.player_pos += self.player_vel * dt;

        if self.player_pos.y < 0.0 {
            self.player_pos.y = 0.0;
            self.player_vel.y = 0.0;
            self.on_ground = true;
        } else {
            self.on_ground = false;
        }
    }

    pub fn player_transform(&self) -> Option<(Vec3, bool)> {
        Some((self.player_pos, self.on_ground))
    }
}