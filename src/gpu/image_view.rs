use super::{HasRawAshHandle, HasRawVkHandle, Image};
use ash::vk;
use std::sync::Arc;

pub struct ImageView {
    image: Arc<Image>,
    vk_image_view: vk::ImageView,
}

impl ImageView {
    pub fn new(
        image: Arc<Image>,
        view_type: vk::ImageViewType,
        format: vk::Format,
        subresource_range: vk::ImageSubresourceRange,
    ) -> Arc<Self> {
        let vk_image_view = unsafe {
            let vk_image_view_info = vk::ImageViewCreateInfo {
                s_type: vk::StructureType::IMAGE_VIEW_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::ImageViewCreateFlags::empty(),
                image: image.get_vk_handle(),
                view_type,
                format,
                components: vk::ComponentMapping::default(),
                subresource_range,
            };

            image
                .device()
                .get_ash_handle()
                .create_image_view(&vk_image_view_info, None)
                .expect("failed to create image view")
        };

        Arc::new(Self {
            image,
            vk_image_view,
        })
    }
}

impl HasRawVkHandle<vk::ImageView> for ImageView {
    unsafe fn get_vk_handle(&self) -> vk::ImageView {
        self.vk_image_view
    }
}

impl Drop for ImageView {
    fn drop(&mut self) {
        unsafe {
            self.image
                .device()
                .get_ash_handle()
                .destroy_image_view(self.vk_image_view, None);
        }
    }
}
