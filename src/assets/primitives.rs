use super::vertex::Vertex;
use super::{Mesh, Model};

/// Unit-up textured ground quad centered on the origin.
pub fn ground_plane(half_extent: f32, uv_repeat: f32) -> Model {
    let n = [0.0, 1.0, 0.0];
    let h = half_extent;
    let u = uv_repeat;
    let vertices = vec![
        Vertex::new([-h, 0.0, -h], n, [0.0, 0.0]),
        Vertex::new([h, 0.0, -h], n, [u, 0.0]),
        Vertex::new([h, 0.0, h], n, [u, u]),
        Vertex::new([-h, 0.0, h], n, [0.0, u]),
    ];
    Model {
        meshes: vec![Mesh {
            vertices,
            // CCW from +Y so the normal points up.
            indices: vec![0, 2, 1, 0, 3, 2],
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_pixels: Some(checker_rgba(64, [0x4a, 0x6b, 0x3a, 0xff], [0x3a, 0x55, 0x2e, 0xff])),
            albedo_size: (64, 64),
        }],
        name: "ground".into(),
        sockets: Vec::new(),
    }
}

/// Simple colored box used for world props when no mesh is assigned.
/// Origin at the bottom center, height 1, footprint 1×1.
pub fn unit_box(color: [f32; 4]) -> Model {
    let positions = [
        [-0.5, 0.0, -0.5],
        [0.5, 0.0, -0.5],
        [0.5, 1.0, -0.5],
        [-0.5, 1.0, -0.5],
        [-0.5, 0.0, 0.5],
        [0.5, 0.0, 0.5],
        [0.5, 1.0, 0.5],
        [-0.5, 1.0, 0.5],
    ];
    let faces: [([u16; 4], [f32; 3]); 6] = [
        ([0, 1, 2, 3], [0.0, 0.0, -1.0]),
        ([5, 4, 7, 6], [0.0, 0.0, 1.0]),
        ([4, 0, 3, 7], [-1.0, 0.0, 0.0]),
        ([1, 5, 6, 2], [1.0, 0.0, 0.0]),
        ([3, 2, 6, 7], [0.0, 1.0, 0.0]),
        ([4, 5, 1, 0], [0.0, -1.0, 0.0]),
    ];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (corners, normal) in faces {
        let base = vertices.len() as u32;
        for i in 0..4 {
            let p = positions[corners[i] as usize];
            let uv = match i {
                0 => [0.0, 1.0],
                1 => [1.0, 1.0],
                2 => [1.0, 0.0],
                _ => [0.0, 0.0],
            };
            vertices.push(Vertex::new(p, normal, uv));
        }
        // Winding chosen so the geometric cross product matches `normal`.
        indices.extend_from_slice(&[base, base + 3, base + 2, base, base + 2, base + 1]);
    }

    Model {
        meshes: vec![Mesh {
            vertices,
            indices,
            albedo: color,
            albedo_pixels: None,
            albedo_size: (1, 1),
        }],
        name: "box".into(),
        sockets: Vec::new(),
    }
}

fn checker_rgba(size: u32, a: [u8; 4], b: [u8; 4]) -> Vec<u8> {
    let mut px = Vec::with_capacity((size * size * 4) as usize);
    let cell = (size / 8).max(1);
    for y in 0..size {
        for x in 0..size {
            let on = ((x / cell) + (y / cell)) % 2 == 0;
            px.extend_from_slice(if on { &a } else { &b });
        }
    }
    px
}
