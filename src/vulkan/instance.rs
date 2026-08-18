use anyhow::Result;
use ash::{khr, vk, Entry, Instance};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::CStr;
use winit::window::Window;

pub struct InstanceBundle {
    pub entry: Entry,
    pub instance: Instance,
    pub surface_loader: khr::surface::Instance,
    pub surface: vk::SurfaceKHR,
    debug: Option<DebugBundle>,
}

struct DebugBundle {
    loader: ash::ext::debug_utils::Instance,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl InstanceBundle {
    pub fn new(window: &Window) -> Result<Self> {
        let entry = Entry::linked();

        let app_name = CStr::from_bytes_with_nul(b"Spacetime\0")?;
        let engine_name = CStr::from_bytes_with_nul(b"spacetime-renderer\0")?;
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name)
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(engine_name)
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_2);

        let mut ext_ptrs =
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?.to_vec();

        let available_exts = unsafe { entry.enumerate_instance_extension_properties(None)? };
        let has_debug_utils = available_exts.iter().any(|e| {
            e.extension_name_as_c_str()
                .ok()
                .is_some_and(|n| n == ash::ext::debug_utils::NAME)
        });

        let enable_validation = cfg!(debug_assertions) && layer_available(&entry, VALIDATION_LAYER);
        if enable_validation && has_debug_utils {
            ext_ptrs.push(ash::ext::debug_utils::NAME.as_ptr());
        }

        let mut layer_ptrs = Vec::new();
        if enable_validation {
            layer_ptrs.push(VALIDATION_LAYER.as_ptr());
            log::info!("enabling {}", VALIDATION_LAYER.to_string_lossy());
        }

        let mut debug_ci = debug_messenger_info();
        let mut create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&ext_ptrs)
            .enabled_layer_names(&layer_ptrs);
        if enable_validation && has_debug_utils {
            create_info = create_info.push_next(&mut debug_ci);
        }

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        let debug = if enable_validation && has_debug_utils {
            let loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let messenger = unsafe { loader.create_debug_utils_messenger(&debug_ci, None)? };
            Some(DebugBundle { loader, messenger })
        } else {
            None
        };

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };
        let surface_loader = khr::surface::Instance::new(&entry, &instance);

        log::info!("Vulkan instance + surface created");
        Ok(Self {
            entry,
            instance,
            surface_loader,
            surface,
            debug,
        })
    }
}

const VALIDATION_LAYER: &CStr = c"VK_LAYER_KHRONOS_validation";

fn layer_available(entry: &Entry, name: &CStr) -> bool {
    match unsafe { entry.enumerate_instance_layer_properties() } {
        Ok(layers) => layers
            .iter()
            .any(|l| l.layer_name_as_c_str().ok() == Some(name)),
        Err(_) => false,
    }
}

fn debug_messenger_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(debug_callback))
}

unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _ty: vk::DebugUtilsMessageTypeFlagsEXT,
    data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user: *mut std::ffi::c_void,
) -> vk::Bool32 {
    if data.is_null() {
        return vk::FALSE;
    }
    let data = &*data;
    let msg = if data.p_message.is_null() {
        std::borrow::Cow::Borrowed("(null)")
    } else {
        CStr::from_ptr(data.p_message).to_string_lossy()
    };
    if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        log::error!("vulkan: {msg}");
    } else {
        log::warn!("vulkan: {msg}");
    }
    vk::FALSE
}

impl Drop for InstanceBundle {
    fn drop(&mut self) {
        unsafe {
            if let Some(dbg) = self.debug.take() {
                dbg.loader.destroy_debug_utils_messenger(dbg.messenger, None);
            }
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
