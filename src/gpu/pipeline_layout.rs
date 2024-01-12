use super::{DescriptorSetLayout, Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::sync::Arc;

pub struct PipelineLayout {
    device: Arc<Device>,
    descriptor_set_layouts: Box<[Arc<DescriptorSetLayout>]>,
    vk_pipeline_layout: vk::PipelineLayout,
}

impl PipelineLayout {
    pub fn new(
        device: Arc<Device>,
        descriptor_set_layouts: &[Arc<DescriptorSetLayout>],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> Arc<PipelineLayout> {
        let mut info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(),
            set_layout_count: 0,
            p_set_layouts: std::ptr::null(),
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };

        let vk_pipeline_layout = unsafe {
            let vk_set_layouts;
            if descriptor_set_layouts.len() > 0 {
                vk_set_layouts = descriptor_set_layouts
                    .iter()
                    .map(|x| x.get_vk_handle())
                    .collect::<Vec<_>>();

                info.set_layout_count = vk_set_layouts.len().try_into().unwrap();
                info.p_set_layouts = vk_set_layouts.as_ptr();
            }

            if push_constant_ranges.len() > 0 {
                info.push_constant_range_count = push_constant_ranges.len().try_into().unwrap();
                info.p_push_constant_ranges = push_constant_ranges.as_ptr();
            }

            device
                .get_ash_handle()
                .create_pipeline_layout(&info, None)
                .expect("failed to create pipeline layout")
        };

        Arc::new(PipelineLayout {
            device,
            descriptor_set_layouts: descriptor_set_layouts.into(),
            vk_pipeline_layout,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl HasRawVkHandle<vk::PipelineLayout> for PipelineLayout {
    unsafe fn get_vk_handle(&self) -> vk::PipelineLayout {
        self.vk_pipeline_layout
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_pipeline_layout(self.vk_pipeline_layout, None);
        }
    }
}
