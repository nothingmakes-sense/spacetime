use glam::Mat4;

use crate::assets::Skeleton;

/// Fold the legs and drop the hips. Used because KayKit has no Sit clip.
pub fn apply_sit(skeleton: &Skeleton, locals: &mut [Mat4], amount: f32) {
    if amount <= 0.001 {
        return;
    }
    let a = amount.clamp(0.0, 1.0);
    let thigh = Mat4::from_rotation_x(1.15 * a);
    let shin = Mat4::from_rotation_x(-1.35 * a);
    for name in ["upperleg.l", "upperleg.r"] {
        if let Some(i) = skeleton.index(name) {
            locals[i] = locals[i] * thigh;
        }
    }
    for name in ["lowerleg.l", "lowerleg.r"] {
        if let Some(i) = skeleton.index(name) {
            locals[i] = locals[i] * shin;
        }
    }
    if let Some(i) = skeleton.index("hips") {
        locals[i] = Mat4::from_translation(glam::Vec3::new(0.0, -0.42 * a, 0.12 * a)) * locals[i];
    }
}
