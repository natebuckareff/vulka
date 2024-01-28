use super::{CommandBuffer, Device, Fence, QueueFamily, Semaphore, Swapchain};
use super::{HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::sync::Arc;

pub struct Queue {
    device: Arc<Device>,
    family_index: u32,
    vk_queue: vk::Queue,
    index: u32,
}

impl Queue {
    pub fn new(
        device: Arc<Device>,
        family_index: u32,
        vk_queue: vk::Queue,
        index: u32,
    ) -> Arc<Queue> {
        Arc::new(Queue {
            device,
            family_index,
            vk_queue,
            index,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn queue_family(&self) -> &QueueFamily {
        let i: usize = self.family_index.try_into().unwrap();
        &self.device.queue_families()[i]
    }

    pub fn submit(
        &self,
        wait: Option<&[(&Semaphore, vk::PipelineStageFlags2)]>,
        command_buffers: &[&CommandBuffer],
        signal: Option<&[(&Semaphore, vk::PipelineStageFlags2)]>,
        fence: Option<&Fence>,
    ) -> () {
        // TODO: This feels like it could be improved. Too much unnecessary
        // copying and `queue_submit` works on batches so the API should
        // probably be batch-oriented

        let mut submit_info = vk::SubmitInfo2::default();
        let mut wait_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = vec![];
        let mut command_buffer_infos: Vec<vk::CommandBufferSubmitInfo> = vec![];
        let mut signal_semaphore_infos: Vec<vk::SemaphoreSubmitInfo> = vec![];

        unsafe {
            if let Some(wait) = wait {
                wait_semaphore_infos.reserve(wait.len());

                for x in wait {
                    let info = vk::SemaphoreSubmitInfo::builder()
                        .semaphore(x.0.get_vk_handle())
                        .value(1)
                        .stage_mask(x.1)
                        .device_index(0)
                        .build();

                    wait_semaphore_infos.push(info);
                }

                submit_info.wait_semaphore_info_count = wait.len().try_into().unwrap();
                submit_info.p_wait_semaphore_infos = wait_semaphore_infos.as_ptr();
            }

            if command_buffers.len() > 0 {
                command_buffer_infos.reserve(command_buffers.len());

                for x in command_buffers {
                    let info = vk::CommandBufferSubmitInfo::builder()
                        .command_buffer(x.get_vk_handle())
                        .device_mask(0)
                        .build();

                    command_buffer_infos.push(info);
                }

                submit_info.command_buffer_info_count = command_buffers.len().try_into().unwrap();
                submit_info.p_command_buffer_infos = command_buffer_infos.as_ptr();
            }

            if let Some(signal) = signal {
                signal_semaphore_infos.reserve(signal.len());

                for x in signal {
                    let info = vk::SemaphoreSubmitInfo::builder()
                        .semaphore(x.0.get_vk_handle())
                        .value(1)
                        .stage_mask(x.1)
                        .device_index(0)
                        .build();

                    signal_semaphore_infos.push(info);
                }

                submit_info.signal_semaphore_info_count = signal.len().try_into().unwrap();
                submit_info.p_signal_semaphore_infos = signal_semaphore_infos.as_ptr();
            }

            let submit_fence = fence
                .map(|x| x.get_vk_handle())
                .unwrap_or(vk::Fence::null());

            self.device
                .get_ash_handle()
                .queue_submit2(self.get_vk_handle(), &[submit_info], submit_fence)
                // .queue_submit(self.vk_queue, submit_infos, submit_fence)
                .expect("failed to submit to queue");
        }
    }

    pub fn submit_present(
        &self,
        wait: &[&Semaphore],
        swapchain: &Swapchain,
        image_index: u32,
    ) -> Result<bool, vk::Result> {
        let mut info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            swapchain_count: 1,
            p_swapchains: std::ptr::null(),
            p_image_indices: std::ptr::null(),
            ..Default::default()
        };

        let mut vk_wait_semaphores: Vec<vk::Semaphore>;
        let vk_swapchains: [vk::SwapchainKHR; 1];

        unsafe {
            if wait.len() > 0 {
                vk_wait_semaphores = vec![];
                vk_wait_semaphores.reserve(wait.len());

                for x in wait {
                    vk_wait_semaphores.push(x.get_vk_handle());
                }

                info.wait_semaphore_count = wait.len().try_into().unwrap();
                info.p_wait_semaphores = vk_wait_semaphores.as_ptr();
            }

            vk_swapchains = [swapchain.get_vk_handle()];
            info.p_swapchains = vk_swapchains.as_ptr();
            info.p_image_indices = &image_index;

            swapchain
                .get_ash_handle()
                .queue_present(self.vk_queue, &info)
        }
    }

    pub fn wait_idle(&self) -> () {
        unsafe {
            self.device
                .get_ash_handle()
                .queue_wait_idle(self.vk_queue)
                .expect("failed to wait for queue to idle")
        };
    }
}

impl HasRawVkHandle<vk::Queue> for Queue {
    unsafe fn get_vk_handle(&self) -> vk::Queue {
        self.vk_queue
    }
}
