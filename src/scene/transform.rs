use glam::{Mat4, Quat, Vec3};

/// TRS every scene object carries. Convert to a model matrix for drawing.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn from_translation(t: Vec3) -> Self {
        Self {
            translation: t,
            ..Self::default()
        }
    }

    pub fn from_yaw(translation: Vec3, yaw: f32) -> Self {
        Self {
            translation,
            rotation: Quat::from_rotation_y(yaw),
            scale: Vec3::ONE,
        }
    }

    pub fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}
