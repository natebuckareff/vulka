use super::{HasRawAshHandle, HasRawVkHandle, PhysicalDevice};
use ash::vk;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::cell::OnceCell;
use std::ffi::CStr;
use std::sync::Arc;

pub struct Instance {
    ash_entry: ash::Entry,
    ash_instance: ash::Instance,
    surface: Surface,
    vk_physical_devices: OnceCell<Vec<vk::PhysicalDevice>>,
}

impl Instance {
    pub fn new(window: &Arc<impl HasRawDisplayHandle + HasRawWindowHandle>) -> Arc<Instance> {
        unsafe {
            let app_info = vk::ApplicationInfo {
                s_type: vk::StructureType::APPLICATION_INFO,
                p_next: std::ptr::null(),
                p_application_name: CStr::from_bytes_with_nul(b"vulka\0").unwrap().as_ptr(),
                application_version: 0,
                p_engine_name: CStr::from_bytes_with_nul(b"no engine\0").unwrap().as_ptr(),
                engine_version: 0,
                api_version: vk::make_api_version(0, 1, 3, 268),
            };

            let mut enabled_layer_names = vec![];

            if cfg!(debug_assertions) {
                enabled_layer_names.push(
                    CStr::from_bytes_with_nul(b"VK_LAYER_KHRONOS_validation\0")
                        .unwrap()
                        .as_ptr(),
                );

                // enabled_layer_names.push(
                //     CStr::from_bytes_with_nul(b"VK_LAYER_LUNARG_api_dump\0")
                //         .unwrap()
                //         .as_ptr(),
                // );
            }

            let raw_display_handle = window.raw_display_handle();
            let raw_window_handle = window.raw_window_handle();

            // Get the necessary extensions for the window surface
            let surface_extensions = ash_window::enumerate_required_extensions(raw_display_handle)
                .expect("failed to get windowing extensions");

            let create_info = vk::InstanceCreateInfo {
                s_type: vk::StructureType::INSTANCE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::InstanceCreateFlags::default(),
                p_application_info: &app_info,
                enabled_layer_count: enabled_layer_names.len().try_into().unwrap(),
                pp_enabled_layer_names: enabled_layer_names.as_ptr(),
                enabled_extension_count: surface_extensions.len().try_into().unwrap(),
                pp_enabled_extension_names: surface_extensions.as_ptr(),
            };

            let ash_entry = ash::Entry::load().expect("failed to initialize ash");

            let ash_instance = ash_entry
                .create_instance(&create_info, None)
                .expect("failed to create instance");

            // Create the window surface handle
            let vk_surface = ash_window::create_surface(
                &ash_entry,
                &ash_instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )
            .expect("failed to create surface");

            let ash_surface_fn = ash::extensions::khr::Surface::new(&ash_entry, &ash_instance);

            Arc::new(Instance {
                ash_entry,
                ash_instance,
                surface: Surface::new(vk_surface, ash_surface_fn),
                vk_physical_devices: OnceCell::new(),
            })
        }
    }

    pub fn get_surface(&self) -> &Surface {
        &self.surface
    }

    fn _get_physical_device_handles(&self) -> &[vk::PhysicalDevice] {
        self.vk_physical_devices
            .get_or_init(|| unsafe { self.ash_instance.enumerate_physical_devices().unwrap() })
    }

    pub fn get_physical_devices(self: &Arc<Instance>) -> Vec<Arc<PhysicalDevice>> {
        self._get_physical_device_handles()
            .iter()
            .map(|vk_phy_device| PhysicalDevice::new(*vk_phy_device, self.clone()))
            .collect()
    }
}

impl HasRawAshHandle<ash::Instance> for Instance {
    unsafe fn get_ash_handle(&self) -> &ash::Instance {
        &self.ash_instance
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.ash_instance.destroy_instance(None);
        }
    }
}

pub struct Surface {
    vk_surface: vk::SurfaceKHR,
    ash_surface_fn: ash::extensions::khr::Surface,
}

impl Surface {
    pub fn new(vk_surface: vk::SurfaceKHR, ash_surface_fn: ash::extensions::khr::Surface) -> Self {
        Self {
            vk_surface,
            ash_surface_fn,
        }
    }
}

impl HasRawAshHandle<ash::extensions::khr::Surface> for Surface {
    unsafe fn get_ash_handle(&self) -> &ash::extensions::khr::Surface {
        &self.ash_surface_fn
    }
}

impl HasRawVkHandle<vk::SurfaceKHR> for Surface {
    unsafe fn get_vk_handle(&self) -> vk::SurfaceKHR {
        self.vk_surface
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.ash_surface_fn.destroy_surface(self.vk_surface, None);
        }
    }
}
