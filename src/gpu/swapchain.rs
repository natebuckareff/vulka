use super::{Device, Fence, HasRawAshHandle, HasRawVkHandle, Image, Semaphore};
use ash::vk;
use std::{cell::OnceCell, sync::Arc};

pub struct Swapchain {
    gpu_device: Arc<Device>,
    vk_swapchain: vk::SwapchainKHR,
    ash_swapchain_fn: ash::extensions::khr::Swapchain,
    format: vk::Format,
    extent: vk::Extent2D,
    images: OnceCell<Vec<Arc<Image>>>,
}

impl Swapchain {
    pub fn new(
        gpu_device: &Arc<Device>,
        min_image_count: u32,
        image_format: vk::Format,
        image_color_space: vk::ColorSpaceKHR,
        image_extent: vk::Extent2D,
        image_usage: vk::ImageUsageFlags,
        present_mode: vk::PresentModeKHR,
        old_swapchain: Option<&Arc<Swapchain>>,
    ) -> Arc<Swapchain> {
        // TODO: Assumes that graphics and presentation queues are the same,
        // which will usually be the case. Should check if they're different and
        // use `vk::SharingMode::CONCURRENT` and pass in `pQueueFamilyIndices`

        let gpu_phy_device = gpu_device.physical_device();
        let gpu_instance = gpu_phy_device.instance();

        let swapchain_create_info = unsafe {
            let cap = gpu_phy_device.get_surface_capabilities();

            let vk_old_swapchain = match old_swapchain {
                None => vk::SwapchainKHR::null(),
                Some(x) => x.vk_swapchain,
            };

            vk::SwapchainCreateInfoKHR {
                s_type: vk::StructureType::SWAPCHAIN_CREATE_INFO_KHR,
                p_next: std::ptr::null(),
                flags: vk::SwapchainCreateFlagsKHR::empty(),
                surface: gpu_instance.get_surface().get_vk_handle(),
                min_image_count,
                image_format,
                image_color_space,
                image_extent,
                image_array_layers: 1,
                image_usage,
                image_sharing_mode: vk::SharingMode::EXCLUSIVE,
                queue_family_index_count: 0,
                p_queue_family_indices: std::ptr::null(),
                pre_transform: cap.current_transform,
                composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                present_mode,
                clipped: vk::TRUE,
                old_swapchain: vk_old_swapchain,
            }
        };

        let ash_swapchain_fn = unsafe {
            let ash_instance = gpu_instance.get_ash_handle();
            let ash_device = gpu_device.get_ash_handle();
            ash::extensions::khr::Swapchain::new(&ash_instance, &ash_device)
        };

        let vk_swapchain = unsafe {
            ash_swapchain_fn
                .create_swapchain(&swapchain_create_info, None)
                .expect("failed to create swapchain")
        };

        Arc::new(Swapchain {
            gpu_device: gpu_device.clone(),
            vk_swapchain,
            ash_swapchain_fn,
            format: image_format,
            extent: image_extent,
            images: OnceCell::new(),
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.gpu_device
    }

    pub fn format(&self) -> &vk::Format {
        &self.format
    }

    pub fn extent(&self) -> &vk::Extent2D {
        &self.extent
    }

    pub fn images(&self) -> &[Arc<Image>] {
        self.images.get_or_init(|| unsafe {
            self.ash_swapchain_fn
                .get_swapchain_images(self.vk_swapchain)
                .unwrap()
                .into_iter()
                .map(|vk_image| Image::new(&self.gpu_device, vk_image, true))
                .collect()
        })
    }

    pub fn acquire_next_image(
        &self,
        timeout: Option<u64>,
        semaphore: Option<&Semaphore>,
        fence: Option<&Fence>,
    ) -> Result<(u32, bool), vk::Result> {
        unsafe {
            let vk_semaphore = semaphore
                .map(|x| x.get_vk_handle())
                .unwrap_or(vk::Semaphore::null());

            let vk_fence = fence
                .map(|x| x.get_vk_handle())
                .unwrap_or(vk::Fence::null());

            self.ash_swapchain_fn.acquire_next_image(
                self.vk_swapchain,
                timeout.unwrap_or(u64::MAX),
                vk_semaphore,
                vk_fence,
            )
        }
    }
}

impl HasRawAshHandle<ash::extensions::khr::Swapchain> for Swapchain {
    unsafe fn get_ash_handle(&self) -> &ash::extensions::khr::Swapchain {
        &self.ash_swapchain_fn
    }
}

impl HasRawVkHandle<vk::SwapchainKHR> for Swapchain {
    unsafe fn get_vk_handle(&self) -> vk::SwapchainKHR {
        self.vk_swapchain
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.ash_swapchain_fn
                .destroy_swapchain(self.vk_swapchain, None);
        }
    }
}
