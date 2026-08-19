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

/// Screen-facing textured quad (XY, +Z). Used for inventory slot sprites.
pub fn sprite_quad(pixels: Vec<u8>, w: u32, h: u32, name: &str) -> Model {
    let n = [0.0, 0.0, 1.0];
    Model {
        meshes: vec![Mesh {
            vertices: vec![
                Vertex::new([-0.5, -0.5, 0.0], n, [0.0, 1.0]),
                Vertex::new([0.5, -0.5, 0.0], n, [1.0, 1.0]),
                Vertex::new([0.5, 0.5, 0.0], n, [1.0, 0.0]),
                Vertex::new([-0.5, 0.5, 0.0], n, [0.0, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_pixels: Some(pixels),
            albedo_size: (w.max(1), h.max(1)),
        }],
        name: name.into(),
        sockets: Vec::new(),
    }
}

/// 5×7 bitmap glyph as a textured quad facing +Z. Used by the F3 debug overlay.
pub fn glyph_quad(ch: char) -> Model {
    let bits = glyph_bits(ch);
    let w = 5u32;
    let h = 7u32;
    let scale = 3u32;
    let tw = w * scale;
    let th = h * scale;
    let mut px = vec![0u8; (tw * th * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            if (bits[y as usize] >> (4 - x)) & 1 == 0 {
                continue;
            }
            for sy in 0..scale {
                for sx in 0..scale {
                    let ix = (x * scale + sx) as usize;
                    let iy = (y * scale + sy) as usize;
                    let i = (iy * tw as usize + ix) * 4;
                    px[i] = 255;
                    px[i + 1] = 255;
                    px[i + 2] = 255;
                    px[i + 3] = 255;
                }
            }
        }
    }
    let n = [0.0, 0.0, 1.0];
    Model {
        meshes: vec![Mesh {
            vertices: vec![
                Vertex::new([-0.5, -0.5, 0.0], n, [0.0, 1.0]),
                Vertex::new([0.5, -0.5, 0.0], n, [1.0, 1.0]),
                Vertex::new([0.5, 0.5, 0.0], n, [1.0, 0.0]),
                Vertex::new([-0.5, 0.5, 0.0], n, [0.0, 0.0]),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            albedo: [1.0, 1.0, 1.0, 1.0],
            albedo_pixels: Some(px),
            albedo_size: (tw, th),
        }],
        name: format!("glyph_{}", ch as u32),
        sockets: Vec::new(),
    }
}

fn glyph_bits(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b01110, 0b10000, 0b11110, 0b10001, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '.' => [0, 0, 0, 0, 0, 0, 0b00100],
        ':' => [0, 0b00100, 0, 0, 0, 0b00100, 0],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        ' ' => [0; 7],
        _ => [0b11111, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11111],
    }
}

