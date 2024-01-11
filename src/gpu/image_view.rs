use super::{HasRawAshHandle, HasRawVkHandle, Image};
use ash::vk;
use std::sync::Arc;

pub struct ImageView {
    gpu_image: Arc<Image>,
    vk_image_view: vk::ImageView,
}

impl ImageView {
    pub fn new(gpu_image: &Arc<Image>, vk_image_view: vk::ImageView) -> Arc<ImageView> {
        Arc::new(ImageView {
            gpu_image: gpu_image.clone(),
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
            self.gpu_image
                .device()
                .get_ash_handle()
                .destroy_image_view(self.vk_image_view, None);
        }
    }
}
