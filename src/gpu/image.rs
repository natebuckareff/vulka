use super::{Device, HasRawAshHandle, HasRawVkHandle, ImageView};
use ash::vk;
use std::sync::Arc;

pub struct Image {
    gpu_device: Arc<Device>,
    vk_image: vk::Image,
    is_swapchain_image: bool,
}

impl Image {
    pub fn new(
        gpu_device: &Arc<Device>,
        vk_image: vk::Image,
        is_swapchain_image: bool,
    ) -> Arc<Image> {
        Arc::new(Image {
            gpu_device: gpu_device.clone(),
            vk_image,
            is_swapchain_image,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.gpu_device
    }

    pub fn get_image_view(
        self: &Arc<Image>,
        view_type: vk::ImageViewType,
        format: vk::Format,
        components: vk::ComponentMapping,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Arc<ImageView> {
        let create_info = vk::ImageViewCreateInfo {
            s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::ImageViewCreateFlags::empty(),
            image: self.vk_image,
            view_type,
            format,
            components,
            subresource_range,
        };

        let vk_image_view = unsafe {
            self.gpu_device
                .get_ash_handle()
                .create_image_view(&create_info, None)
                .expect("failed to create image view")
        };

        ImageView::new(&self, vk_image_view)
    }
}

impl HasRawVkHandle<vk::Image> for Image {
    unsafe fn get_vk_handle(&self) -> vk::Image {
        self.vk_image
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if !self.is_swapchain_image {
            unsafe {
                self.gpu_device
                    .get_ash_handle()
                    .destroy_image(self.vk_image, None);
            }
        }
    }
}
