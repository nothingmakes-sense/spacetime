use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct CameraUbo {
    pub view: [[f32; 4]; 4],
    pub proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _pad: f32,
}

impl CameraUbo {
    pub fn new(view: Mat4, proj: Mat4, eye: Vec3) -> Self {
        // glam::perspective_rh is Y-up with +Y → +clip.y. Vulkan NDC is Y-down,
        // so without this the world is drawn upside-down (ground in the sky).
        let mut proj = proj;
        proj.y_axis.y *= -1.0;
        Self {
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
            camera_pos: eye.to_array(),
            _pad: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LightUbo {
    pub light_pos: [f32; 3],
    pub ambient_strength: f32,
    pub light_color: [f32; 3],
    pub specular_strength: f32,
    pub shininess: f32,
    pub _pad: [f32; 3],
}

impl LightUbo {
    pub fn new(
        pos: Vec3,
        color: Vec3,
        ambient: f32,
        specular: f32,
        shininess: f32,
    ) -> Self {
        Self {
            light_pos: pos.to_array(),
            ambient_strength: ambient,
            light_color: color.to_array(),
            specular_strength: specular,
            shininess,
            _pad: [0.0; 3],
        }
    }
}
