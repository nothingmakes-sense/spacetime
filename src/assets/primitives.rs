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

/// Chest body (0.8 × 0.45 × 0.55) and a thin lid that hinges at the back.
pub fn chest_parts() -> (Model, Model) {
    let body = scaled_box(0.80, 0.45, 0.55, [0.45, 0.28, 0.14, 1.0], "chest_body");
    let mut lid = scaled_box(0.82, 0.06, 0.57, [0.55, 0.34, 0.16, 1.0], "chest_lid");
    // Sit the lid on top of the body.
    for v in &mut lid.meshes[0].vertices {
        v.position[1] += 0.45;
    }
    (body, lid)
}

/// Stone furnace body + a small ember cube parented near the mouth.
pub fn furnace_parts() -> (Model, Model) {
    let body = scaled_box(0.72, 1.05, 0.72, [0.28, 0.26, 0.24, 1.0], "furnace");
    let ember = scaled_box(1.0, 1.0, 1.0, [1.0, 0.42, 0.08, 1.0], "furnace_ember");
    (body, ember)
}

/// Wide wooden crafting table.
pub fn workbench_model() -> Model {
    scaled_box(1.35, 0.78, 0.75, [0.50, 0.32, 0.16, 1.0], "workbench")
}

/// HUD slot plate (flat, origin at center).
pub fn slot_plate(color: [f32; 4], name: &str) -> Model {
    let mut m = scaled_box(1.0, 0.08, 1.0, color, name);
    for mesh in &mut m.meshes {
        for v in &mut mesh.vertices {
            v.position[1] -= 0.04;
        }
    }
    m
}

/// Tiny billboard-ish cube used as a world item gem.
pub fn item_gem(color: [f32; 4]) -> Model {
    scaled_box(1.0, 1.0, 1.0, color, "item")
}

/// 3×5 bitmap digit as a textured quad facing +Z, origin at center.
pub fn digit_quad(digit: u8) -> Model {
    let bits = DIGITS[(digit as usize).min(9)];
    let w = 3u32;
    let h = 5u32;
    let scale = 4u32;
    let tw = w * scale;
    let th = h * scale;
    let mut px = vec![0u8; (tw * th * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let on = (bits[y as usize] >> (2 - x)) & 1 == 1;
            if !on {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let ix = (x * scale + sx) as usize;
                    let iy = (y * scale + sy) as usize;
                    let i = (iy * tw as usize + ix) * 4;
                    px[i] = 255;
                    px[i + 1] = 255;
                    px[i + 2] = 240;
                    px[i + 3] = 255;
                }
            }
        }
    }
    let hw = 0.5;
    let hh = 0.5;
    let n = [0.0, 0.0, 1.0];
    Model {
        meshes: vec![Mesh {
            vertices: vec![
                Vertex::new([-hw, -hh, 0.0], n, [0.0, 1.0]),
                Vertex::new([hw, -hh, 0.0], n, [1.0, 1.0]),
                Vertex::new([hw, hh, 0.0], n, [1.0, 0.0]),
                Vertex::new([-hw, hh, 0.0], n, [0.0, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_pixels: Some(px),
            albedo_size: (tw, th),
        }],
        name: format!("digit_{digit}"),
        sockets: Vec::new(),
    }
}

const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b001, 0b001], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

pub fn scaled_box(sx: f32, sy: f32, sz: f32, color: [f32; 4], name: &str) -> Model {
    let mut m = unit_box(color);
    for mesh in &mut m.meshes {
        for v in &mut mesh.vertices {
            v.position[0] *= sx;
            v.position[1] *= sy;
            v.position[2] *= sz;
        }
    }
    m.name = name.into();
    m
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
