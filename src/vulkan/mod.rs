//! Modular Vulkan renderer: instance → device → swapchain → pipeline → draw.

mod device;
mod instance;
mod memory;
mod pipeline;
mod swapchain;
mod sync;
mod texture;
mod ubo;

use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use ash::vk;
use glam::{Mat4, Vec3};
use winit::window::Window;

use crate::assets::Model;
use device::DeviceBundle;
use instance::InstanceBundle;
use memory::AllocatedBuffer;
use pipeline::PipelineBundle;
use swapchain::{pick_surface_format, SwapchainBundle};
use sync::FrameSync;
use ubo::{CameraUbo, LightUbo};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

#[derive(Clone, Copy, Debug)]
pub struct ModelHandle(pub usize);

struct GpuSubmesh {
    vertices: AllocatedBuffer,
    indices: AllocatedBuffer,
    index_count: u32,
    material_set: vk::DescriptorSet,
    texture: memory::AllocatedImage,
}

struct GpuModel {
    parts: Vec<GpuSubmesh>,
}

pub struct VulkanContext {
    window: Arc<Window>,
    pipeline: PipelineBundle,
    swapchain: SwapchainBundle,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    sync: FrameSync,

    descriptor_pool: vk::DescriptorPool,
    global_sets: Vec<vk::DescriptorSet>,
    camera_ubos: Vec<AllocatedBuffer>,
    light_ubos: Vec<AllocatedBuffer>,
    overlay_camera: AllocatedBuffer,
    overlay_light: AllocatedBuffer,
    overlay_set: vk::DescriptorSet,
    sampler: vk::Sampler,

    models: Vec<GpuModel>,

    current_frame: usize,
    recording_image: Option<u32>,
    skip_frame: bool,
    resized: bool,

    pending_camera: CameraUbo,
    pending_light: LightUbo,
    pub vsync: bool,

    /// Device must drop before instance. Keep these last.
    device: DeviceBundle,
    instance: InstanceBundle,
}

impl VulkanContext {
    pub fn new(window: Arc<Window>, vsync: bool) -> Result<Self> {
        let instance = InstanceBundle::new(&window)?;
        let device = DeviceBundle::new(
            &instance.instance,
            &instance.surface_loader,
            instance.surface,
        )?;

        let formats = unsafe {
            instance.surface_loader.get_physical_device_surface_formats(
                device.physical,
                instance.surface,
            )?
        };
        let color_format = pick_surface_format(&formats).format;

        let pipeline = PipelineBundle::create(&device.device, color_format)?;
        let swapchain = SwapchainBundle::create(
            &instance.instance,
            &device.device,
            device.physical,
            &instance.surface_loader,
            instance.surface,
            &device.swapchain_loader,
            &window,
            pipeline.render_pass,
            device.graphics_family,
            device.present_family,
            vsync,
            None,
        )?;

        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(device.graphics_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.device.create_command_pool(&pool_info, None)? };

        let alloc = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(MAX_FRAMES_IN_FLIGHT as u32);
        let command_buffers = unsafe { device.device.allocate_command_buffers(&alloc)? };

        let sync = FrameSync::new(&device.device, swapchain.images.len())?;

        let sampler = texture::create_sampler(&device.device)?;

        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 256,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 4096,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: 4096,
            },
        ];
        let pool_ci = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(4096)
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);
        let descriptor_pool = unsafe { device.device.create_descriptor_pool(&pool_ci, None)? };

        let mut camera_ubos = Vec::new();
        let mut light_ubos = Vec::new();
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            camera_ubos.push(memory::create_buffer(
                &instance.instance,
                &device.device,
                device.physical,
                std::mem::size_of::<CameraUbo>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?);
            light_ubos.push(memory::create_buffer(
                &instance.instance,
                &device.device,
                device.physical,
                std::mem::size_of::<LightUbo>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?);
        }

        let layouts = vec![pipeline.global_set_layout; MAX_FRAMES_IN_FLIGHT];
        let alloc_sets = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);
        let global_sets = unsafe { device.device.allocate_descriptor_sets(&alloc_sets)? };

        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let cam_info = vk::DescriptorBufferInfo::default()
                .buffer(camera_ubos[i].buffer)
                .offset(0)
                .range(std::mem::size_of::<CameraUbo>() as u64);
            let light_info = vk::DescriptorBufferInfo::default()
                .buffer(light_ubos[i].buffer)
                .offset(0)
                .range(std::mem::size_of::<LightUbo>() as u64);
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(global_sets[i])
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&cam_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(global_sets[i])
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&light_info)),
            ];
            unsafe { device.device.update_descriptor_sets(&writes, &[]) };
        }

        let overlay_camera = memory::create_buffer(
            &instance.instance,
            &device.device,
            device.physical,
            std::mem::size_of::<CameraUbo>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        let overlay_light = memory::create_buffer(
            &instance.instance,
            &device.device,
            device.physical,
            std::mem::size_of::<LightUbo>() as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        memory::copy_to_buffer(&device.device, &overlay_camera, &[CameraUbo::overlay()]);
        memory::copy_to_buffer(&device.device, &overlay_light, &[LightUbo::overlay()]);

        let overlay_layouts = [pipeline.global_set_layout];
        let overlay_alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&overlay_layouts);
        let overlay_set = unsafe { device.device.allocate_descriptor_sets(&overlay_alloc)? }[0];
        {
            let cam_info = vk::DescriptorBufferInfo::default()
                .buffer(overlay_camera.buffer)
                .offset(0)
                .range(std::mem::size_of::<CameraUbo>() as u64);
            let light_info = vk::DescriptorBufferInfo::default()
                .buffer(overlay_light.buffer)
                .offset(0)
                .range(std::mem::size_of::<LightUbo>() as u64);
            let writes = [
                vk::WriteDescriptorSet::default()
                    .dst_set(overlay_set)
                    .dst_binding(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&cam_info)),
                vk::WriteDescriptorSet::default()
                    .dst_set(overlay_set)
                    .dst_binding(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(std::slice::from_ref(&light_info)),
            ];
            unsafe { device.device.update_descriptor_sets(&writes, &[]) };
        }

        log::info!("Vulkan renderer ready");
        Ok(Self {
            window,
            pipeline,
            swapchain,
            command_pool,
            command_buffers,
            sync,
            descriptor_pool,
            global_sets,
            camera_ubos,
            light_ubos,
            overlay_camera,
            overlay_light,
            overlay_set,
            sampler,
            models: Vec::new(),
            current_frame: 0,
            recording_image: None,
            skip_frame: false,
            resized: false,
            pending_camera: CameraUbo::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO),
            pending_light: LightUbo::new(Vec3::Y * 10.0, Vec3::ONE, 0.2, 0.4, 32.0),
            vsync,
            device,
            instance,
        })
    }

    pub fn upload_model(&mut self, model: &Model) -> Result<ModelHandle> {
        let mut parts = Vec::new();
        for mesh in &model.meshes {
            if mesh.vertices.is_empty() || mesh.indices.is_empty() {
                log::warn!("skipping empty submesh on '{}'", model.name);
                continue;
            }
            match self.upload_submesh(mesh) {
                Ok(part) => parts.push(part),
                Err(e) => {
                    unsafe {
                        for p in &parts {
                            memory::destroy_buffer(&self.device.device, &p.vertices);
                            memory::destroy_buffer(&self.device.device, &p.indices);
                            memory::destroy_image(&self.device.device, &p.texture);
                        }
                    }
                    return Err(e.context(format!("submesh of '{}'", model.name)));
                }
            }
        }

        if parts.is_empty() {
            return Err(anyhow!("model '{}' has no drawable parts", model.name));
        }

        let handle = ModelHandle(self.models.len());
        log::info!(
            "uploaded model '{}' ({} parts, {} verts)",
            model.name,
            parts.len(),
            model.vertex_count()
        );
        self.models.push(GpuModel { parts });
        Ok(handle)
    }

    fn upload_submesh(&self, mesh: &crate::assets::Mesh) -> Result<GpuSubmesh> {
        let vsize = (mesh.vertices.len() * std::mem::size_of::<crate::assets::Vertex>()) as u64;
        let isize = (mesh.indices.len() * std::mem::size_of::<u32>()) as u64;

        let vertices = memory::create_buffer(
            &self.instance.instance,
            &self.device.device,
            self.device.physical,
            vsize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        memory::copy_to_buffer(&self.device.device, &vertices, &mesh.vertices);

        let indices = match memory::create_buffer(
            &self.instance.instance,
            &self.device.device,
            self.device.physical,
            isize,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        ) {
            Ok(b) => b,
            Err(e) => {
                unsafe { memory::destroy_buffer(&self.device.device, &vertices) };
                return Err(e);
            }
        };
        memory::copy_to_buffer(&self.device.device, &indices, &mesh.indices);

        let solid;
        let (px, w, h) = if let Some(ref pixels) = mesh.albedo_pixels {
            (pixels.as_slice(), mesh.albedo_size.0, mesh.albedo_size.1)
        } else {
            let c = mesh.albedo;
            solid = [
                (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                (c[3].clamp(0.0, 1.0) * 255.0) as u8,
            ];
            (solid.as_slice(), 1, 1)
        };

        let tex = match texture::create_texture(
            &self.instance.instance,
            &self.device.device,
            self.device.physical,
            self.device.graphics_queue,
            self.command_pool,
            px,
            w.max(1),
            h.max(1),
        ) {
            Ok(t) => t,
            Err(e) => {
                unsafe {
                    memory::destroy_buffer(&self.device.device, &vertices);
                    memory::destroy_buffer(&self.device.device, &indices);
                }
                return Err(e);
            }
        };

        let layouts = [self.pipeline.material_set_layout];
        let alloc = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);
        let set = match unsafe { self.device.device.allocate_descriptor_sets(&alloc) } {
            Ok(s) => s[0],
            Err(e) => {
                unsafe {
                    memory::destroy_buffer(&self.device.device, &vertices);
                    memory::destroy_buffer(&self.device.device, &indices);
                    memory::destroy_image(&self.device.device, &tex);
                }
                return Err(anyhow!("descriptor set: {e}"));
            }
        };

        let image_info = vk::DescriptorImageInfo::default()
            .image_view(tex.view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let sampler_info = vk::DescriptorImageInfo::default().sampler(self.sampler);
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(std::slice::from_ref(&image_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(std::slice::from_ref(&sampler_info)),
        ];
        unsafe { self.device.device.update_descriptor_sets(&writes, &[]) };

        Ok(GpuSubmesh {
            vertices,
            indices,
            index_count: mesh.indices.len() as u32,
            material_set: set,
            texture: tex,
        })
    }

    /// Rewrite host-visible vertex buffers after CPU skinning.
    pub fn update_model_vertices(
        &mut self,
        handle: ModelHandle,
        parts: &[Vec<crate::assets::Vertex>],
    ) {
        let Some(gpu) = self.models.get(handle.0) else {
            return;
        };
        let n = gpu.parts.len().min(parts.len());
        for i in 0..n {
            if parts[i].is_empty() {
                continue;
            }
            memory::copy_to_buffer(&self.device.device, &gpu.parts[i].vertices, &parts[i]);
        }
    }

    pub fn begin_frame(&mut self) -> Result<()> {
        self.skip_frame = false;
        self.recording_image = None;

        if self.resized {
            self.resized = false;
            self.recreate_swapchain_internal()?;
            self.skip_frame = true;
            return Ok(());
        }

        let frame = self.current_frame;
        let device = &self.device.device;
        unsafe {
            device.wait_for_fences(&[self.sync.in_flight[frame]], true, u64::MAX)?;
        }

        let acquire = unsafe {
            self.device.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                u64::MAX,
                self.sync.image_available[frame],
                vk::Fence::null(),
            )
        };

        let image_index = match acquire {
            Ok((idx, suboptimal)) => {
                if suboptimal {
                    self.resized = true;
                }
                idx
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain_internal()?;
                self.skip_frame = true;
                return Ok(());
            }
            Err(e) => return Err(anyhow!("acquire_next_image: {e}")),
        };

        let image_fence = self.sync.image_in_flight[image_index as usize];
        if image_fence != vk::Fence::null() {
            unsafe { device.wait_for_fences(&[image_fence], true, u64::MAX)? };
        }
        self.sync.image_in_flight[image_index as usize] = self.sync.in_flight[frame];

        unsafe {
            device.reset_fences(&[self.sync.in_flight[frame]])?;
            device.reset_command_buffer(
                self.command_buffers[frame],
                vk::CommandBufferResetFlags::empty(),
            )?;
        }

        memory::copy_to_buffer(device, &self.camera_ubos[frame], &[self.pending_camera]);
        memory::copy_to_buffer(device, &self.light_ubos[frame], &[self.pending_light]);

        let cmd = self.command_buffers[frame];
        let begin = vk::CommandBufferBeginInfo::default();
        unsafe { device.begin_command_buffer(cmd, &begin)? };

        let clear = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.05, 0.07, 0.10, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        let rp = vk::RenderPassBeginInfo::default()
            .render_pass(self.pipeline.render_pass)
            .framebuffer(self.swapchain.framebuffers[image_index as usize])
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            })
            .clear_values(&clear);

        unsafe {
            device.cmd_begin_render_pass(cmd, &rp, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, self.pipeline.pipeline);

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.extent.width as f32,
                height: self.swapchain.extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.extent,
            };
            device.cmd_set_viewport(cmd, 0, &[viewport]);
            device.cmd_set_scissor(cmd, 0, &[scissor]);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &[self.global_sets[frame]],
                &[],
            );
        }

        self.recording_image = Some(image_index);
        Ok(())
    }

    pub fn update_camera_ubo(&mut self, view: Mat4, proj: Mat4, eye: Vec3) {
        self.pending_camera = CameraUbo::new(view, proj, eye);
    }

    pub fn update_light_ubo(
        &mut self,
        pos: Vec3,
        color: Vec3,
        ambient: f32,
        specular: f32,
        shininess: f32,
    ) {
        self.pending_light = LightUbo::new(pos, color, ambient, specular, shininess);
    }

    /// Switch to the screen-space overlay: identity camera, no depth test,
    /// alpha blend. Everything drawn after this is HUD and cannot be buried
    /// by world geometry.
    pub fn begin_overlay(&mut self) {
        if self.skip_frame || self.recording_image.is_none() {
            return;
        }
        let cmd = self.command_buffers[self.current_frame];
        unsafe {
            self.device.device.cmd_bind_pipeline(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.overlay_pipeline,
            );
            self.device.device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.layout,
                0,
                &[self.overlay_set],
                &[],
            );
        }
    }

    pub fn draw_model(&mut self, handle: ModelHandle, model: Mat4) -> Result<()> {
        if self.skip_frame {
            return Ok(());
        }
        let Some(_) = self.recording_image else {
            return Ok(());
        };
        let gpu = self
            .models
            .get(handle.0)
            .ok_or_else(|| anyhow!("invalid model handle {}", handle.0))?;

        let cmd = self.command_buffers[self.current_frame];
        let device = &self.device.device;
        let model_cols = model.to_cols_array();

        for part in &gpu.parts {
            unsafe {
                device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline.layout,
                    1,
                    &[part.material_set],
                    &[],
                );
                device.cmd_bind_vertex_buffers(cmd, 0, &[part.vertices.buffer], &[0]);
                device.cmd_bind_index_buffer(cmd, part.indices.buffer, 0, vk::IndexType::UINT32);
                device.cmd_push_constants(
                    cmd,
                    self.pipeline.layout,
                    vk::ShaderStageFlags::VERTEX,
                    0,
                    bytemuck::cast_slice(&model_cols),
                );
                device.cmd_draw_indexed(cmd, part.index_count, 1, 0, 0, 0);
            }
        }
        Ok(())
    }

    pub fn end_frame_and_present(&mut self) -> Result<()> {
        if self.skip_frame || self.recording_image.is_none() {
            return Ok(());
        }
        let frame = self.current_frame;
        let image_index = self.recording_image.unwrap();
        let cmd = self.command_buffers[frame];
        let device = &self.device.device;

        unsafe {
            device.cmd_end_render_pass(cmd);
            device.end_command_buffer(cmd)?;
        }

        let wait = [self.sync.image_available[frame]];
        let stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal = [self.sync.render_finished[image_index as usize]];
        let cmds = [cmd];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&wait)
            .wait_dst_stage_mask(&stages)
            .command_buffers(&cmds)
            .signal_semaphores(&signal);

        unsafe {
            device.queue_submit(
                self.device.graphics_queue,
                &[submit],
                self.sync.in_flight[frame],
            )?;
        }

        let swaps = [self.swapchain.swapchain];
        let indices = [image_index];
        let present = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal)
            .swapchains(&swaps)
            .image_indices(&indices);

        let result = unsafe {
            self.device
                .swapchain_loader
                .queue_present(self.device.present_queue, &present)
        };
        match result {
            Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.resized = true,
            Ok(false) => {}
            Err(e) => return Err(anyhow!("present: {e}")),
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        self.recording_image = None;
        Ok(())
    }

    pub fn recreate_swapchain(&mut self, _w: u32, _h: u32) -> Result<()> {
        self.resized = true;
        Ok(())
    }

    pub fn set_vsync(&mut self, vsync: bool) {
        if self.vsync != vsync {
            self.vsync = vsync;
            self.resized = true;
        }
    }

    fn recreate_swapchain_internal(&mut self) -> Result<()> {
        let size = self.window.inner_size();
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        unsafe { self.device.device.device_wait_idle()? };

        let old = self.swapchain.swapchain;
        let new = SwapchainBundle::create(
            &self.instance.instance,
            &self.device.device,
            self.device.physical,
            &self.instance.surface_loader,
            self.instance.surface,
            &self.device.swapchain_loader,
            &self.window,
            self.pipeline.render_pass,
            self.device.graphics_family,
            self.device.present_family,
            self.vsync,
            Some(old),
        )?;
        unsafe {
            self.swapchain
                .destroy(&self.device.device, &self.device.swapchain_loader);
        }
        self.swapchain = new;
        self.sync
            .resize_images(&self.device.device, self.swapchain.images.len())?;
        log::info!("swapchain recreated {}x{}", size.width, size.height);
        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device.device_wait_idle();
            for m in &self.models {
                for p in &m.parts {
                    memory::destroy_buffer(&self.device.device, &p.vertices);
                    memory::destroy_buffer(&self.device.device, &p.indices);
                    memory::destroy_image(&self.device.device, &p.texture);
                }
            }
            self.sync.destroy(&self.device.device);
            for b in &self.camera_ubos {
                memory::destroy_buffer(&self.device.device, b);
            }
            for b in &self.light_ubos {
                memory::destroy_buffer(&self.device.device, b);
            }
            memory::destroy_buffer(&self.device.device, &self.overlay_camera);
            memory::destroy_buffer(&self.device.device, &self.overlay_light);
            self.device.device.destroy_sampler(self.sampler, None);
            self.device
                .device
                .destroy_command_pool(self.command_pool, None);
            self.device
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.swapchain
                .destroy(&self.device.device, &self.device.swapchain_loader);
            self.pipeline.destroy(&self.device.device);
        }
    }
}
