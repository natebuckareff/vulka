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
        device: &Arc<Device>,
        family_index: u32,
        vk_queue: vk::Queue,
        index: u32,
    ) -> Arc<Queue> {
        Arc::new(Queue {
            device: device.clone(),
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
        wait: &[(&Semaphore, vk::PipelineStageFlags)],
        command_buffers: &[&CommandBuffer],
        signal: Option<&[&Semaphore]>,
        fence: Option<&Fence>,
    ) -> () {
        // TODO: This feels like it could be improved. Too much unnecessary
        // copying and `queue_submit` works on batches so the API should
        // probably be batch-oriented

        let mut info = vk::SubmitInfo {
            s_type: vk::StructureType::SUBMIT_INFO,
            p_next: std::ptr::null(),
            wait_semaphore_count: 0,
            p_wait_semaphores: std::ptr::null(),
            p_wait_dst_stage_mask: std::ptr::null(),
            command_buffer_count: 0,
            p_command_buffers: std::ptr::null(),
            signal_semaphore_count: 0,
            p_signal_semaphores: std::ptr::null(),
        };

        let mut vk_wait_semaphores: Vec<vk::Semaphore>;
        let mut vk_wait_dst_stage_mask: Vec<vk::PipelineStageFlags>;
        let mut vk_command_buffers: Vec<vk::CommandBuffer>;
        let mut vk_signal_semaphores: Vec<vk::Semaphore>;

        unsafe {
            if wait.len() > 0 {
                vk_wait_semaphores = vec![];
                vk_wait_semaphores.reserve(wait.len());

                vk_wait_dst_stage_mask = vec![];
                vk_wait_dst_stage_mask.reserve(wait.len());

                for x in wait {
                    vk_wait_semaphores.push(x.0.get_vk_handle());
                    vk_wait_dst_stage_mask.push(x.1);
                }

                info.wait_semaphore_count = wait.len().try_into().unwrap();
                info.p_wait_semaphores = vk_wait_semaphores.as_ptr();
                info.p_wait_dst_stage_mask = vk_wait_dst_stage_mask.as_ptr();
            }

            if command_buffers.len() > 0 {
                vk_command_buffers = vec![];
                vk_command_buffers.reserve(command_buffers.len());

                for x in command_buffers {
                    vk_command_buffers.push(x.get_vk_handle());
                }

                info.command_buffer_count = command_buffers.len().try_into().unwrap();
                info.p_command_buffers = vk_command_buffers.as_ptr();
            }

            if let Some(signal) = signal {
                vk_signal_semaphores = vec![];
                vk_signal_semaphores.reserve(signal.len());

                for x in signal {
                    vk_signal_semaphores.push(x.get_vk_handle());
                }

                info.signal_semaphore_count = signal.len().try_into().unwrap();
                info.p_signal_semaphores = vk_signal_semaphores.as_ptr();
            }

            let submit_infos = &[info];

            let submit_fence = fence
                .map(|x| x.get_vk_handle())
                .unwrap_or(vk::Fence::null());

            self.device
                .get_ash_handle()
                .queue_submit(self.vk_queue, submit_infos, submit_fence)
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
}

impl HasRawVkHandle<vk::Queue> for Queue {
    unsafe fn get_vk_handle(&self) -> vk::Queue {
        self.vk_queue
    }
}
