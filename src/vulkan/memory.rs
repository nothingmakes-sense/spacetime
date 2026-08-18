use anyhow::{anyhow, Result};
use ash::{vk, Device};

pub struct AllocatedBuffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    pub size: vk::DeviceSize,
}

pub struct AllocatedImage {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub width: u32,
    pub height: u32,
}

pub fn find_memory_type(
    instance: &ash::Instance,
    physical: vk::PhysicalDevice,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> Result<u32> {
    let mem = unsafe { instance.get_physical_device_memory_properties(physical) };
    for i in 0..mem.memory_type_count {
        if type_filter & (1 << i) != 0
            && mem.memory_types[i as usize]
                .property_flags
                .contains(properties)
        {
            return Ok(i);
        }
    }
    Err(anyhow!("no suitable memory type (filter={type_filter:#x})"))
}

pub fn create_buffer(
    instance: &ash::Instance,
    device: &Device,
    physical: vk::PhysicalDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<AllocatedBuffer> {
    let info = vk::BufferCreateInfo::default()
        .size(size.max(1))
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let buffer = unsafe { device.create_buffer(&info, None)? };
    let req = unsafe { device.get_buffer_memory_requirements(buffer) };
    let memory_type = find_memory_type(instance, physical, req.memory_type_bits, properties)?;
    let alloc = vk::MemoryAllocateInfo::default()
        .allocation_size(req.size)
        .memory_type_index(memory_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_buffer_memory(buffer, memory, 0)? };
    Ok(AllocatedBuffer {
        buffer,
        memory,
        size: req.size,
    })
}

pub fn create_image_2d(
    instance: &ash::Instance,
    device: &Device,
    physical: vk::PhysicalDevice,
    width: u32,
    height: u32,
    format: vk::Format,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    properties: vk::MemoryPropertyFlags,
    aspect: vk::ImageAspectFlags,
) -> Result<AllocatedImage> {
    let info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .format(format)
        .tiling(tiling)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(usage)
        .samples(vk::SampleCountFlags::TYPE_1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let image = unsafe { device.create_image(&info, None)? };
    let req = unsafe { device.get_image_memory_requirements(image) };
    let memory_type = find_memory_type(instance, physical, req.memory_type_bits, properties)?;
    let alloc = vk::MemoryAllocateInfo::default()
        .allocation_size(req.size)
        .memory_type_index(memory_type);
    let memory = unsafe { device.allocate_memory(&alloc, None)? };
    unsafe { device.bind_image_memory(image, memory, 0)? };

    let view_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: aspect,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    let view = unsafe { device.create_image_view(&view_info, None)? };

    Ok(AllocatedImage {
        image,
        memory,
        view,
        width,
        height,
    })
}

pub unsafe fn destroy_buffer(device: &Device, buf: &AllocatedBuffer) {
    device.destroy_buffer(buf.buffer, None);
    device.free_memory(buf.memory, None);
}

pub unsafe fn destroy_image(device: &Device, img: &AllocatedImage) {
    device.destroy_image_view(img.view, None);
    device.destroy_image(img.image, None);
    device.free_memory(img.memory, None);
}

pub fn copy_to_buffer<T: bytemuck::Pod>(device: &Device, buf: &AllocatedBuffer, data: &[T]) {
    let bytes = bytemuck::cast_slice(data);
    unsafe {
        let ptr = device
            .map_memory(buf.memory, 0, buf.size, vk::MemoryMapFlags::empty())
            .expect("map memory");
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
        device.unmap_memory(buf.memory);
    }
}
