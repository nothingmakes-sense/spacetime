//! Frame-in-flight + per-swapchain-image synchronization.
//!
//! Binary present semaphores cannot be shared across swapchain images: the
//! presentation engine may still be waiting on a semaphore after `queue_present`
//! returns. See https://docs.vulkan.org/guide/latest/swapchain_semaphore_reuse.html

use anyhow::Result;
use ash::{vk, Device};

use super::MAX_FRAMES_IN_FLIGHT;

pub struct FrameSync {
    /// Signaled by `acquire_next_image`, waited on by submit. One per frame.
    pub image_available: Vec<vk::Semaphore>,
    /// Signaled by submit, waited on by present. One per swapchain image.
    pub render_finished: Vec<vk::Semaphore>,
    /// Signaled when submit for that frame slot is done. One per frame.
    pub in_flight: Vec<vk::Fence>,
    /// Fence that last submitted work for each swapchain image.
    pub image_in_flight: Vec<vk::Fence>,
}

impl FrameSync {
    pub fn new(device: &Device, image_count: usize) -> Result<Self> {
        let sem_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        let mut image_available = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut in_flight = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            unsafe {
                image_available.push(device.create_semaphore(&sem_info, None)?);
                in_flight.push(device.create_fence(&fence_info, None)?);
            }
        }

        let render_finished = create_semaphores(device, image_count)?;
        let image_in_flight = vec![vk::Fence::null(); image_count];

        Ok(Self {
            image_available,
            render_finished,
            in_flight,
            image_in_flight,
        })
    }

    /// Rebuild present semaphores after the swapchain image count changes.
    /// Caller must `device_wait_idle` first.
    pub fn resize_images(&mut self, device: &Device, image_count: usize) -> Result<()> {
        unsafe {
            for s in self.render_finished.drain(..) {
                device.destroy_semaphore(s, None);
            }
        }
        self.render_finished = create_semaphores(device, image_count)?;
        self.image_in_flight = vec![vk::Fence::null(); image_count];
        Ok(())
    }

    pub unsafe fn destroy(&mut self, device: &Device) {
        for s in self.image_available.drain(..) {
            device.destroy_semaphore(s, None);
        }
        for s in self.render_finished.drain(..) {
            device.destroy_semaphore(s, None);
        }
        for f in self.in_flight.drain(..) {
            device.destroy_fence(f, None);
        }
        self.image_in_flight.clear();
    }
}

fn create_semaphores(device: &Device, count: usize) -> Result<Vec<vk::Semaphore>> {
    let info = vk::SemaphoreCreateInfo::default();
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(unsafe { device.create_semaphore(&info, None)? });
    }
    Ok(out)
}
