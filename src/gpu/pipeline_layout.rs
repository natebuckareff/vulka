use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::sync::Arc;

pub struct PipelineLayout {
    device: Arc<Device>,
    vk_pipeline_layout: vk::PipelineLayout,
}

impl PipelineLayout {
    pub fn new(
        device: &Arc<Device>,
        descriptor_set_layouts: Option<&[()]>,
        push_constant_ranges: Option<&[vk::PushConstantRange]>,
    ) -> Arc<PipelineLayout> {
        let mut pipeline_layout_create_info = vk::PipelineLayoutCreateInfo {
            s_type: vk::StructureType::PIPELINE_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineLayoutCreateFlags::empty(),
            set_layout_count: 0,
            p_set_layouts: std::ptr::null(),
            push_constant_range_count: 0,
            p_push_constant_ranges: std::ptr::null(),
        };

        if let Some(_descriptor_set_layouts) = descriptor_set_layouts {
            todo!()
        }

        if let Some(push_constant_ranges) = push_constant_ranges {
            pipeline_layout_create_info.push_constant_range_count =
                push_constant_ranges.len().try_into().unwrap();

            pipeline_layout_create_info.p_push_constant_ranges = push_constant_ranges.as_ptr();
        }

        let vk_pipeline_layout = unsafe {
            device
                .get_ash_handle()
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .expect("failed to create pipeline layout")
        };

        Arc::new(PipelineLayout {
            device: device.clone(),
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
