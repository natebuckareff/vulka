use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::sync::Arc;

pub struct Semaphore {
    device: Arc<Device>,
    vk_semaphore: vk::Semaphore,
}

impl Semaphore {
    pub fn new(device: Arc<Device>) -> Self {
        let vk_semaphore = unsafe {
            device
                .get_ash_handle()
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                .expect("failed to create semaphore")
        };
        Self {
            device,
            vk_semaphore,
        }
    }
}

impl HasRawVkHandle<vk::Semaphore> for Semaphore {
    unsafe fn get_vk_handle(&self) -> vk::Semaphore {
        self.vk_semaphore
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_semaphore(self.vk_semaphore, None);
        }
    }
}

pub struct Fence {
    device: Arc<Device>,
    vk_fence: vk::Fence,
}

impl Fence {
    pub fn new(device: Arc<Device>) -> Self {
        let vk_fence = unsafe {
            device
                .get_ash_handle()
                .create_fence(&vk::FenceCreateInfo::default(), None)
                .expect("failed to create fence")
        };
        Self {
            device,
            vk_fence,
        }
    }

    pub fn signaled(device: Arc<Device>) -> Self {
        let vk_fence = unsafe {
            device
                .get_ash_handle()
                .create_fence(
                    &vk::FenceCreateInfo {
                        flags: vk::FenceCreateFlags::SIGNALED,
                        ..Default::default()
                    },
                    None,
                )
                .expect("failed to create fence")
        };
        Self {
            device,
            vk_fence,
        }
    }
}

impl HasRawVkHandle<vk::Fence> for Fence {
    unsafe fn get_vk_handle(&self) -> vk::Fence {
        self.vk_fence
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_fence(self.vk_fence, None);
        }
    }
}
