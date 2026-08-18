use glam::Vec3;

pub struct WorldEntity {
    pub position: Vec3,
    pub half_extents: Vec3,
    // mesh id, color, etc.
}

pub struct LocalWorld {
    pub entities: Vec<WorldEntity>,
}

impl LocalWorld {
    pub fn generate_basic() -> Self {
        let mut entities = Vec::new();
        // a few boxes scattered around
        for i in -3..=3 {
            entities.push(WorldEntity {
                position: Vec3::new(i as f32 * 4.0, 0.5, -6.0),
                half_extents: Vec3::splat(0.5),
            });
        }
        Self { entities }
    }
}