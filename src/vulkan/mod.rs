use anyhow::Result;
use glam::{Mat4, Vec3};
use std::sync::Arc;
use winit::window::Window;

use crate::assets::Model;

pub struct VulkanContext {
    // Real ash handles will go here later
}

impl VulkanContext {
    pub fn new(_window: Arc<Window>) -> Result<Self> {
        // TODO: full ash initialization (instance, device, swapchain, etc.)
        log::warn!("VulkanContext is currently a stub – rendering will not work yet");
        Ok(Self {})
    }

    pub fn upload_model(&mut self, _model: &Model) -> Result<()> {
        Ok(())
    }

    pub fn begin_frame(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn update_camera_ubo(&mut self, _view: Mat4, _proj: Mat4, _eye: Vec3) {}

    pub fn update_light_ubo(
        &mut self,
        _pos: Vec3,
        _color: Vec3,
        _ambient: f32,
        _specular: f32,
        _shininess: f32,
    ) {
    }

    pub fn draw_model(&mut self, _model: &Model, _model_matrix: Mat4) -> Result<()> {
        Ok(())
    }

    pub fn end_frame_and_present(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn recreate_swapchain(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }
}