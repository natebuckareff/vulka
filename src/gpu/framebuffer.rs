use super::{HasRawAshHandle, HasRawVkHandle, ImageView, RenderPass};
use ash::vk;
use std::sync::Arc;

pub struct Framebuffer {
    render_pass: Arc<RenderPass>,
    vk_framebuffer: vk::Framebuffer,
    image_views: Vec<Arc<ImageView>>,
}

impl Framebuffer {
    pub fn new(
        render_pass: &Arc<RenderPass>,
        image_views: Vec<Arc<ImageView>>,
        width: u32,
        height: u32,
        layers: u32,
    ) -> Arc<Framebuffer> {
        assert!(image_views.len() == render_pass.attachment_count() as usize);

        let vk_image_views: Vec<_> =
            unsafe { image_views.iter().map(|x| x.get_vk_handle()).collect() };

        let create_info = unsafe {
            vk::FramebufferCreateInfo {
                s_type: vk::StructureType::FRAMEBUFFER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::FramebufferCreateFlags::empty(),
                render_pass: render_pass.get_vk_handle(),
                attachment_count: render_pass.attachment_count(),
                p_attachments: vk_image_views.as_ptr(),
                width,
                height,
                layers,
            }
        };

        let vk_framebuffer = unsafe {
            render_pass
                .device()
                .get_ash_handle()
                .create_framebuffer(&create_info, None)
                .expect("failed to create framebuffer")
        };

        Arc::new(Framebuffer {
            render_pass: render_pass.clone(),
            vk_framebuffer,
            image_views,
        })
    }

    pub fn render_pass(&self) -> &Arc<RenderPass> {
        &self.render_pass
    }
}

impl HasRawVkHandle<vk::Framebuffer> for Framebuffer {
    unsafe fn get_vk_handle(&self) -> vk::Framebuffer {
        self.vk_framebuffer
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.render_pass
                .device()
                .get_ash_handle()
                .destroy_framebuffer(self.vk_framebuffer, None);
        }
    }
}
