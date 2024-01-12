use super::{Fence, HasRawAshHandle, HasRawVkHandle, PhysicalDevice, Queue, Swapchain};
use ash::vk;
use std::cell::OnceCell;
use std::ffi::CStr;
use std::sync::{Arc, Weak};

pub struct Device {
    gpu_phy_device: Arc<PhysicalDevice>,
    vk_phy_device: vk::PhysicalDevice,
    ash_device: ash::Device,
    queue_families: Vec<QueueFamily>,
}

impl Device {
    pub fn new(
        gpu_phy_device: &Arc<PhysicalDevice>,
        vk_phy_device: vk::PhysicalDevice,
        queue_family_indices: &[u32],
        enabled_extensions: &[&[u8]],
    ) -> Arc<Device> {
        // Get the filtered list of queue families
        let queue_family_properties = gpu_phy_device.get_queue_family_properties();
        let mut queue_family_configs = vec![];
        for index in queue_family_indices {
            let i: usize = (*index).try_into().unwrap();
            queue_family_configs.push(QueueFamilyConfig::new(*index, queue_family_properties[i]));
        }

        // `queue_create_infos` must not outlive `queue_families`
        let mut queue_create_infos = vec![];
        for queue_family in &queue_family_configs {
            queue_create_infos.push(unsafe { queue_family.get_device_queue_create_info() });
        }

        let enabled_extensions_ptrs = enabled_extensions
            .iter()
            .map(|x| CStr::from_bytes_with_nul(x).unwrap().as_ptr())
            .collect::<Vec<_>>();

        let enabled_features = vk::PhysicalDeviceFeatures {
            ..Default::default()
        };

        let device_create_info = vk::DeviceCreateInfo {
            s_type: vk::StructureType::DEVICE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceCreateFlags::empty(),
            queue_create_info_count: queue_create_infos.len().try_into().unwrap(),
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_extension_count: enabled_extensions_ptrs.len().try_into().unwrap(),
            pp_enabled_extension_names: enabled_extensions_ptrs.as_ptr(),
            p_enabled_features: &enabled_features,
            ..Default::default()
        };

        let ash_device = unsafe {
            let ash_instance = gpu_phy_device.instance().get_ash_handle();
            ash_instance
                .create_device(vk_phy_device, &device_create_info, None)
                .expect("failed to create device")
        };

        Arc::new_cyclic(|arc| Device {
            gpu_phy_device: gpu_phy_device.clone(),
            vk_phy_device,
            ash_device,
            queue_families: queue_family_configs
                .drain(..)
                .map(|x| QueueFamily::new(arc, x))
                .collect(),
        })
    }

    pub fn physical_device(&self) -> &Arc<PhysicalDevice> {
        &self.gpu_phy_device
    }

    pub fn queue_families<'t>(self: &'t Arc<Device>) -> &'t Vec<QueueFamily> {
        &self.queue_families
    }

    pub fn get_first_queue(&self, flags: vk::QueueFlags) -> Option<&Queue> {
        for family in &self.queue_families {
            let queue_flags = family.properties().queue_flags;
            if queue_flags.contains(flags) {
                return Some(family.get_queue(0));
            }
        }
        None
    }

    pub fn get_first_present_queue(&self) -> Option<&Queue> {
        for family in &self.queue_families {
            if family.supports_surface() {
                return Some(family.get_queue(0));
            }
        }
        None
    }

    // TODO
    pub fn get_swapchain(
        self: &Arc<Device>,
        min_image_count: u32,
        image_format: vk::Format,
        image_color_space: vk::ColorSpaceKHR,
        image_extent: vk::Extent2D,
        image_usage: vk::ImageUsageFlags,
        present_mode: vk::PresentModeKHR,
        old_swapchain: Option<&Arc<Swapchain>>,
    ) -> Arc<Swapchain> {
        Swapchain::new(
            self,
            min_image_count,
            image_format,
            image_color_space,
            image_extent,
            image_usage,
            present_mode,
            old_swapchain,
        )
    }

    pub fn wait_idle(&self) -> () {
        unsafe {
            self.ash_device
                .device_wait_idle()
                .expect("faileded waiting for device idle");
        }
    }

    pub fn wait_for_fences(&self, fences: &[&Fence], wait_all: bool, timeout: Option<u64>) -> () {
        unsafe {
            let vk_fences: Vec<_> = fences.iter().map(|x| x.get_vk_handle()).collect();
            self.ash_device
                .wait_for_fences(vk_fences.as_slice(), wait_all, timeout.unwrap_or(u64::MAX))
                .expect("failed to wait for fences")
        }
    }

    pub fn reset_fences(&self, fences: &[&Fence]) -> () {
        unsafe {
            let vk_fences: Vec<_> = fences.iter().map(|x| x.get_vk_handle()).collect();
            self.ash_device
                .reset_fences(vk_fences.as_slice())
                .expect("failed to wait for fences")
        }
    }
}

impl HasRawAshHandle<ash::Device> for Device {
    unsafe fn get_ash_handle(&self) -> &ash::Device {
        &self.ash_device
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.ash_device.destroy_device(None);
        }
    }
}

pub struct QueueFamilyConfig {
    pub index: u32,
    pub properties: vk::QueueFamilyProperties,
    pub priorities: Vec<f32>,
}

impl QueueFamilyConfig {
    pub fn new(index: u32, properties: vk::QueueFamilyProperties) -> QueueFamilyConfig {
        QueueFamilyConfig {
            index,
            properties,
            priorities: vec![1.0; properties.queue_count as usize],
        }
    }

    // Unsafe because `p_queue_priorities` can outlive self
    pub unsafe fn get_device_queue_create_info(&self) -> vk::DeviceQueueCreateInfo {
        vk::DeviceQueueCreateInfo {
            s_type: vk::StructureType::DEVICE_QUEUE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::DeviceQueueCreateFlags::empty(),
            queue_family_index: self.index,
            queue_count: self.properties.queue_count,
            p_queue_priorities: self.priorities.as_ptr(),
        }
    }
}

pub struct QueueFamily {
    device: Weak<Device>,
    config: QueueFamilyConfig,
    queues: Vec<OnceCell<Arc<Queue>>>,
}

impl QueueFamily {
    pub fn new(device: &Weak<Device>, config: QueueFamilyConfig) -> QueueFamily {
        let mut queues: Vec<OnceCell<Arc<Queue>>> = vec![];
        queues.resize_with(config.properties.queue_count.try_into().unwrap(), || {
            OnceCell::new()
        });
        QueueFamily {
            device: device.clone(),
            config,
            queues,
        }
    }

    pub fn index(&self) -> u32 {
        self.config.index
    }

    pub fn properties(&self) -> &vk::QueueFamilyProperties {
        &self.config.properties
    }

    pub fn priorities(&self) -> &[f32] {
        &self.config.priorities
    }

    pub fn supports_surface(&self) -> bool {
        let device_arc = self.device.upgrade().unwrap();
        let physical_device = device_arc.physical_device();
        physical_device.supports_surface(self.index())
    }

    pub fn get_queue(&self, index: u32) -> &Arc<Queue> {
        assert!(index < self.config.properties.queue_count);
        let i: usize = index.try_into().unwrap();
        self.queues[i].get_or_init(|| {
            let family_index = self.config.index;
            let device_arc = self.device.upgrade().unwrap();
            let vk_queue = unsafe {
                device_arc
                    .get_ash_handle()
                    .get_device_queue(family_index, index)
            };
            Queue::new(device_arc, family_index, vk_queue, index)
        })
    }
}
