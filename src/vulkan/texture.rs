use anyhow::{Context, Result};
use ash::{vk, Device};

use super::memory::{self, AllocatedBuffer, AllocatedImage};

pub fn create_texture(
    instance: &ash::Instance,
    device: &Device,
    physical: vk::PhysicalDevice,
    graphics_queue: vk::Queue,
    command_pool: vk::CommandPool,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<AllocatedImage> {
    let size = (width as vk::DeviceSize) * (height as vk::DeviceSize) * 4;
    let staging = memory::create_buffer(
        instance,
        device,
        physical,
        size.max(4),
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;
    memory::copy_to_buffer(device, &staging, pixels);

    let image = match memory::create_image_2d(
        instance,
        device,
        physical,
        width.max(1),
        height.max(1),
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageTiling::OPTIMAL,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        vk::ImageAspectFlags::COLOR,
    ) {
        Ok(img) => img,
        Err(e) => {
            unsafe { memory::destroy_buffer(device, &staging) };
            return Err(e);
        }
    };

    let upload = (|| {
        transition(
            device,
            graphics_queue,
            command_pool,
            image.image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;
        copy_buffer_to_image(device, graphics_queue, command_pool, &staging, &image)?;
        transition(
            device,
            graphics_queue,
            command_pool,
            image.image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;
        Ok::<(), anyhow::Error>(())
    })();

    unsafe { memory::destroy_buffer(device, &staging) };
    if let Err(e) = upload {
        unsafe { memory::destroy_image(device, &image) };
        return Err(e).context("texture upload");
    }
    Ok(image)
}

pub fn create_sampler(device: &Device) -> Result<vk::Sampler> {
    let info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::REPEAT)
        .address_mode_v(vk::SamplerAddressMode::REPEAT)
        .address_mode_w(vk::SamplerAddressMode::REPEAT)
        .anisotropy_enable(true)
        .max_anisotropy(8.0)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK);
    Ok(unsafe { device.create_sampler(&info, None)? })
}

fn single_shot<F>(device: &Device, queue: vk::Queue, pool: vk::CommandPool, f: F) -> Result<()>
where
    F: FnOnce(vk::CommandBuffer),
{
    let alloc = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);
    let cmd = unsafe { device.allocate_command_buffers(&alloc)? }[0];
    let begin = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe {
        device.begin_command_buffer(cmd, &begin)?;
        f(cmd);
        device.end_command_buffer(cmd)?;
        let submit = vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));
        device.queue_submit(queue, &[submit], vk::Fence::null())?;
        device.queue_wait_idle(queue)?;
        device.free_command_buffers(pool, &[cmd]);
    }
    Ok(())
}

fn transition(
    device: &Device,
    queue: vk::Queue,
    pool: vk::CommandPool,
    image: vk::Image,
    old: vk::ImageLayout,
    new: vk::ImageLayout,
) -> Result<()> {
    let (src_access, dst_access, src_stage, dst_stage) =
        if old == vk::ImageLayout::UNDEFINED && new == vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            )
        } else {
            (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            )
        };

    single_shot(device, queue, pool, |cmd| {
        let barrier = vk::ImageMemoryBarrier::default()
            .old_layout(old)
            .new_layout(new)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(src_access)
            .dst_access_mask(dst_access);
        unsafe {
            device.cmd_pipeline_barrier(
                cmd,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    })
}

fn copy_buffer_to_image(
    device: &Device,
    queue: vk::Queue,
    pool: vk::CommandPool,
    src: &AllocatedBuffer,
    dst: &AllocatedImage,
) -> Result<()> {
    single_shot(device, queue, pool, |cmd| {
        let region = vk::BufferImageCopy::default()
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_extent(vk::Extent3D {
                width: dst.width,
                height: dst.height,
                depth: 1,
            });
        unsafe {
            device.cmd_copy_buffer_to_image(
                cmd,
                src.buffer,
                dst.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
    })
}
