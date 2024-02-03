use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::sync::Arc;

pub struct Sampler {
    device: Arc<Device>,
    vk_sampler: vk::Sampler,
}

impl Sampler {
    pub fn new(device: Arc<Device>) -> Arc<Self> {
        let physical_device = device.physical_device();

        let max_anisotropy = physical_device.device_limits().max_sampler_anisotropy;

        let create_info = vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::TRUE,
            max_anisotropy,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
        };

        let vk_sampler = unsafe {
            device
                .get_ash_handle()
                .create_sampler(&create_info, None)
                .unwrap()
        };

        Arc::new(Self { device, vk_sampler })
    }
}

impl HasRawVkHandle<vk::Sampler> for Sampler {
    unsafe fn get_vk_handle(&self) -> vk::Sampler {
        self.vk_sampler
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_sampler(self.vk_sampler, None);
        }
    }
}
