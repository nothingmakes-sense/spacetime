use anyhow::Result;
use ash::{khr, vk, Device};
use winit::window::Window;

use super::memory::{self, AllocatedImage};

pub struct SwapchainBundle {
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub views: Vec<vk::ImageView>,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub depth: AllocatedImage,
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl SwapchainBundle {
    pub fn create(
        instance: &ash::Instance,
        device: &Device,
        physical: vk::PhysicalDevice,
        surface_loader: &khr::surface::Instance,
        surface: vk::SurfaceKHR,
        swapchain_loader: &khr::swapchain::Device,
        window: &Window,
        render_pass: vk::RenderPass,
        graphics_family: u32,
        present_family: u32,
        vsync: bool,
        old: Option<vk::SwapchainKHR>,
    ) -> Result<Self> {
        let caps =
            unsafe { surface_loader.get_physical_device_surface_capabilities(physical, surface)? };
        let formats =
            unsafe { surface_loader.get_physical_device_surface_formats(physical, surface)? };
        let present_modes =
            unsafe { surface_loader.get_physical_device_surface_present_modes(physical, surface)? };

        let surface_format = pick_surface_format(&formats);

        let present_mode = pick_present_mode(&present_modes, vsync);

        let size = window.inner_size();
        let extent = vk::Extent2D {
            width: size.width.clamp(
                caps.min_image_extent.width.max(1),
                caps.max_image_extent.width.max(1),
            ),
            height: size.height.clamp(
                caps.min_image_extent.height.max(1),
                caps.max_image_extent.height.max(1),
            ),
        };

        let mut image_count = caps.min_image_count + 1;
        if caps.max_image_count > 0 && image_count > caps.max_image_count {
            image_count = caps.max_image_count;
        }

        let queue_families = [graphics_family, present_family];
        let mut info = vk::SwapchainCreateInfoKHR::default()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);
        if graphics_family == present_family {
            info = info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        } else {
            info = info
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_families);
        }
        if let Some(old) = old {
            info = info.old_swapchain(old);
        }

        let swapchain = unsafe { swapchain_loader.create_swapchain(&info, None)? };
        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let views = create_views(device, &images, surface_format.format)?;

        let depth = memory::create_image_2d(
            instance,
            device,
            physical,
            extent.width,
            extent.height,
            vk::Format::D32_SFLOAT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            vk::ImageAspectFlags::DEPTH,
        )?;

        let framebuffers = create_framebuffers(device, &views, depth.view, render_pass, extent)?;

        Ok(Self {
            swapchain,
            images,
            views,
            format: surface_format.format,
            extent,
            depth,
            framebuffers,
        })
    }

    pub unsafe fn destroy(&mut self, device: &Device, swapchain_loader: &khr::swapchain::Device) {
        for fb in self.framebuffers.drain(..) {
            device.destroy_framebuffer(fb, None);
        }
        for view in self.views.drain(..) {
            device.destroy_image_view(view, None);
        }
        memory::destroy_image(device, &self.depth);
        swapchain_loader.destroy_swapchain(self.swapchain, None);
    }
}

pub fn pick_present_mode(modes: &[vk::PresentModeKHR], vsync: bool) -> vk::PresentModeKHR {
    if vsync {
        return vk::PresentModeKHR::FIFO;
    }
    if modes.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else if modes.contains(&vk::PresentModeKHR::IMMEDIATE) {
        vk::PresentModeKHR::IMMEDIATE
    } else {
        vk::PresentModeKHR::FIFO
    }
}

pub fn pick_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    formats
        .iter()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .copied()
        .unwrap_or(formats[0])
}

fn create_views(
    device: &Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>> {
    images
        .iter()
        .map(|&image| {
            let info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            Ok(unsafe { device.create_image_view(&info, None)? })
        })
        .collect()
}

fn create_framebuffers(
    device: &Device,
    views: &[vk::ImageView],
    depth_view: vk::ImageView,
    render_pass: vk::RenderPass,
    extent: vk::Extent2D,
) -> Result<Vec<vk::Framebuffer>> {
    views
        .iter()
        .map(|&color| {
            let attachments = [color, depth_view];
            let info = vk::FramebufferCreateInfo::default()
                .render_pass(render_pass)
                .attachments(&attachments)
                .width(extent.width)
                .height(extent.height)
                .layers(1);
            Ok(unsafe { device.create_framebuffer(&info, None)? })
        })
        .collect()
}
