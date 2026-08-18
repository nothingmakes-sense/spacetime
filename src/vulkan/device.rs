use anyhow::{Context, Result};
use ash::{khr, vk, Device, Instance};

pub struct DeviceBundle {
    pub physical: vk::PhysicalDevice,
    pub device: Device,
    pub graphics_family: u32,
    pub present_family: u32,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub swapchain_loader: khr::swapchain::Device,
}

impl DeviceBundle {
    pub fn new(
        instance: &Instance,
        surface_loader: &khr::surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> Result<Self> {
        let physicals = unsafe { instance.enumerate_physical_devices()? };
        let (physical, graphics_family, present_family) = physicals
            .into_iter()
            .find_map(|pd| pick_device(instance, surface_loader, surface, pd))
            .context("no GPU with graphics + present support")?;

        let mut families = vec![graphics_family];
        if present_family != graphics_family {
            families.push(present_family);
        }
        let priorities = [1.0f32];
        let queue_infos: Vec<_> = families
            .iter()
            .map(|&idx| {
                vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(idx)
                    .queue_priorities(&priorities)
            })
            .collect();

        let mut features = vk::PhysicalDeviceFeatures::default();
        features.sampler_anisotropy = vk::TRUE;

        let ext_names = [khr::swapchain::NAME.as_ptr()];
        let create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_infos)
            .enabled_extension_names(&ext_names)
            .enabled_features(&features);

        let device = unsafe { instance.create_device(physical, &create_info, None)? };
        let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_family, 0) };
        let swapchain_loader = khr::swapchain::Device::new(instance, &device);

        let props = unsafe { instance.get_physical_device_properties(physical) };
        let name = props
            .device_name_as_c_str()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown GPU".into());
        log::info!("logical device '{name}' (gfx={graphics_family} present={present_family})");
        Ok(Self {
            physical,
            device,
            graphics_family,
            present_family,
            graphics_queue,
            present_queue,
            swapchain_loader,
        })
    }
}

fn pick_device(
    instance: &Instance,
    surface_loader: &khr::surface::Instance,
    surface: vk::SurfaceKHR,
    pd: vk::PhysicalDevice,
) -> Option<(vk::PhysicalDevice, u32, u32)> {
    let exts = unsafe { instance.enumerate_device_extension_properties(pd).ok()? };
    let has_swapchain = exts.iter().any(|e| {
        e.extension_name_as_c_str()
            .ok()
            .is_some_and(|n| n == khr::swapchain::NAME)
    });
    if !has_swapchain {
        return None;
    }

    let props = unsafe { instance.get_physical_device_queue_family_properties(pd) };
    let mut graphics = None;
    let mut present = None;
    for (i, fam) in props.iter().enumerate() {
        let idx = i as u32;
        if fam.queue_flags.contains(vk::QueueFlags::GRAPHICS) && graphics.is_none() {
            graphics = Some(idx);
        }
        let ok = unsafe {
            surface_loader
                .get_physical_device_surface_support(pd, idx, surface)
                .unwrap_or(false)
        };
        if ok && present.is_none() {
            present = Some(idx);
        }
        // Prefer a family that can do both.
        if fam.queue_flags.contains(vk::QueueFlags::GRAPHICS) && ok {
            graphics = Some(idx);
            present = Some(idx);
            break;
        }
    }
    Some((pd, graphics?, present?))
}

impl Drop for DeviceBundle {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
    }
}
