use super::{Device, HasRawAshHandle, HasRawVkHandle, Instance};
use ash::vk;
use std::cell::OnceCell;
use std::collections::HashSet;
use std::ffi::CStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct PhysicalDevice {
    gpu_instance: Arc<Instance>,
    vk_phy_device: vk::PhysicalDevice,
    // TODO: Factor these out into separate structs so they don't take up memory
    // for entire lifetime of GpuPhysicalDevice?
    properties: OnceCell<vk::PhysicalDeviceProperties>,
    extension_properties: OnceCell<Vec<vk::ExtensionProperties>>,
    extension_names: OnceCell<Vec<Vec<u8>>>,
}

impl PhysicalDevice {
    pub fn new(
        vk_phy_device: vk::PhysicalDevice,
        gpu_instance: Arc<Instance>,
    ) -> Arc<PhysicalDevice> {
        Arc::new(PhysicalDevice {
            gpu_instance: gpu_instance,
            vk_phy_device,
            properties: OnceCell::new(),
            extension_properties: OnceCell::new(),
            extension_names: OnceCell::new(),
        })
    }

    pub fn instance(&self) -> &Arc<Instance> {
        &self.gpu_instance
    }

    pub fn get_device(
        self: &Arc<PhysicalDevice>,
        queue_family_indices: &[u32],
        enabled_extensions: &[&[u8]],
    ) -> Arc<Device> {
        Device::new(
            self.clone(),
            self.vk_phy_device,
            queue_family_indices,
            enabled_extensions,
        )
    }

    fn _get_physical_device_properties(&self) -> &vk::PhysicalDeviceProperties {
        self.properties.get_or_init(|| unsafe {
            self.gpu_instance
                .get_ash_handle()
                .get_physical_device_properties(self.vk_phy_device)
        })
    }

    fn _get_device_extension_properties(&self) -> &[vk::ExtensionProperties] {
        self.extension_properties.get_or_init(|| unsafe {
            self.gpu_instance
                .get_ash_handle()
                .enumerate_device_extension_properties(self.vk_phy_device)
                .unwrap()
        })
    }

    pub fn device_id(&self) -> u32 {
        self._get_physical_device_properties().device_id
    }

    pub fn device_name(&self) -> &str {
        get_str_from_chars(
            &self._get_physical_device_properties()
                .device_name
        )
    }

    pub fn device_type(&self) -> vk::PhysicalDeviceType {
        self._get_physical_device_properties().device_type
    }

    pub fn extension_names(&self) -> &[Vec<u8>] {
        self.extension_names.get_or_init(|| {
            let mut extension_names = vec![];
            for x in self._get_device_extension_properties() {
                let length = x.extension_name.iter().position(|&ch| ch == 0).unwrap() + 1;
                let bytes = unsafe { core::slice::from_raw_parts(x.extension_name.as_ptr() as *const u8, length) };
                extension_names.push(Vec::from(bytes));
            }
            extension_names
        })
    }

    pub fn extension_name_hashset(&self) -> HashSet<&[u8]> {
        HashSet::from_iter(self.extension_names().iter().map(Vec::as_slice))
    }

    pub fn get_queue_family_properties(&self) -> Vec<vk::QueueFamilyProperties> {
        unsafe {
            self.gpu_instance
                .get_ash_handle()
                .get_physical_device_queue_family_properties(self.vk_phy_device)
        }
    }

    pub fn supports_surface(&self, queue_family_index: u32) -> bool {
        unsafe {
            let surface = self.gpu_instance.get_surface();
            surface
                .get_ash_handle()
                .get_physical_device_surface_support(
                    self.vk_phy_device,
                    queue_family_index,
                    surface.get_vk_handle(),
                )
                .unwrap()
        }
    }

    pub fn get_surface_formats(&self) -> Vec<vk::SurfaceFormatKHR> {
        unsafe {
            let surface = self.gpu_instance.get_surface();
            surface
                .get_ash_handle()
                .get_physical_device_surface_formats(self.vk_phy_device, surface.get_vk_handle())
                .unwrap()
        }
    }

    pub fn get_surface_present_modes(&self) -> Vec<vk::PresentModeKHR> {
        unsafe {
            let surface = self.gpu_instance.get_surface();
            surface
                .get_ash_handle()
                .get_physical_device_surface_present_modes(
                    self.vk_phy_device,
                    surface.get_vk_handle(),
                )
                .unwrap()
        }
    }

    pub fn get_surface_capabilities(&self) -> vk::SurfaceCapabilitiesKHR {
        unsafe {
            let surface = self.gpu_instance.get_surface();
            surface
                .get_ash_handle()
                .get_physical_device_surface_capabilities(
                    self.vk_phy_device,
                    surface.get_vk_handle(),
                )
                .unwrap()
        }
    }

    pub fn get_surface_current_extent_clamped(&self, width: u32, height: u32) -> vk::Extent2D {
        let caps = self.get_surface_capabilities();
        let current_extent = caps.current_extent;

        if current_extent.width != u32::MAX && current_extent.height != u32::MAX {
            return current_extent;
        }

        vk::Extent2D {
            width: width.clamp(caps.min_image_extent.width, caps.max_image_extent.width),
            height: height.clamp(caps.min_image_extent.height, caps.max_image_extent.height),
        }
    }

    pub fn get_surface_ideal_image_count(&self) -> u32 {
        let caps = self.get_surface_capabilities();
        let image_count = caps.min_image_count + 1;
        if caps.max_image_count > 0 && image_count > caps.max_image_count {
            return caps.max_image_count;
        }
        image_count
    }

    pub fn get_memory_properties(&self) -> vk::PhysicalDeviceMemoryProperties {
        unsafe {
            self.gpu_instance
                .get_ash_handle()
                .get_physical_device_memory_properties(self.vk_phy_device)
        }
    }
}

impl HasRawVkHandle<vk::PhysicalDevice> for PhysicalDevice {
    unsafe fn get_vk_handle(&self) -> vk::PhysicalDevice {
        self.vk_phy_device
    }
}

fn get_str_from_chars(chars: &[i8]) -> &str {
    let bytes = unsafe { core::slice::from_raw_parts(chars.as_ptr() as *const u8, chars.len()) };
    let cstr = CStr::from_bytes_until_nul(bytes).unwrap();
    cstr.to_str().unwrap()
}

fn get_string_from_chars(chars: &[i8]) -> String {
    String::from(get_str_from_chars(chars))
}
