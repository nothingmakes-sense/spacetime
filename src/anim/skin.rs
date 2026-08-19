use glam::{Mat4, Vec3};

use crate::assets::{SkinnedPrim, Vertex};

pub fn skin_primitive(prim: &SkinnedPrim, palette: &[Mat4]) -> Vec<Vertex> {
    let mut out = Vec::with_capacity(prim.positions.len());
    for i in 0..prim.positions.len() {
        let p = prim.positions[i];
        let n = prim.normals[i];
        let js = prim.joints.get(i).copied().unwrap_or([0; 4]);
        let ws = prim.weights.get(i).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
        let (sp, sn) = skin_vertex(p, n, js, ws, palette);
        out.push(Vertex::new(
            sp.to_array(),
            sn.to_array(),
            prim.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
        ));
    }
    out
}

fn skin_vertex(
    position: Vec3,
    normal: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
    palette: &[glam::Mat4],
) -> (Vec3, Vec3) {
    let mut skinned_p = Vec3::ZERO;
    let mut skinned_n = Vec3::ZERO;
    let mut wsum = 0.0_f32;
    for i in 0..4 {
        let w = weights[i];
        if w <= 0.0 {
            continue;
        }
        let ji = joints[i] as usize;
        if ji >= palette.len() {
            continue;
        }
        let m = palette[ji];
        skinned_p += w * m.transform_point3(position);
        skinned_n += w * m.transform_vector3(normal);
        wsum += w;
    }
    if wsum <= 0.0 {
        return (position, normal);
    }
    (skinned_p / wsum, skinned_n.normalize_or_zero())
}
