use super::{
    Device, Framebuffer, HasRawAshHandle, HasRawVkHandle, Pipeline, QueueFamily, RenderPass,
};
use ash::vk;
use std::rc::Rc;
use std::sync::Arc;

pub struct CommandBufferPool {
    device: Arc<Device>,
    vk_command_pool: vk::CommandPool,
}

impl CommandBufferPool {
    pub fn new(
        device: &Arc<Device>,
        queue_family: &QueueFamily,
        flags: vk::CommandPoolCreateFlags,
    ) -> Rc<Self> {
        let create_info = vk::CommandPoolCreateInfo {
            s_type: vk::StructureType::COMMAND_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags,
            queue_family_index: queue_family.index(),
        };

        let vk_command_pool = unsafe {
            device
                .get_ash_handle()
                .create_command_pool(&create_info, None)
                .expect("failed to create command buffer pool")
        };

        Rc::new(Self {
            device: device.clone(),
            vk_command_pool,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub unsafe fn handle(&self) -> vk::CommandPool {
        self.vk_command_pool
    }

    pub fn allocate_one(
        self: &Rc<CommandBufferPool>,
        level: vk::CommandBufferLevel,
    ) -> CommandBuffer {
        let allocate_info = vk::CommandBufferAllocateInfo {
            s_type: vk::StructureType::COMMAND_BUFFER_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            command_pool: self.vk_command_pool,
            level,
            command_buffer_count: 1,
        };

        let vk_command_buffer = unsafe {
            self.device
                .get_ash_handle()
                .allocate_command_buffers(&allocate_info)
                .unwrap()[0]
        };

        CommandBuffer::new(self, vk_command_buffer)
    }
}

impl HasRawVkHandle<vk::CommandPool> for CommandBufferPool {
    unsafe fn get_vk_handle(&self) -> vk::CommandPool {
        self.vk_command_pool
    }
}

impl Drop for CommandBufferPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_command_pool(self.vk_command_pool, None);
        }
    }
}

pub struct CommandBuffer {
    pool: Rc<CommandBufferPool>,
    vk_command_buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn new(pool: &Rc<CommandBufferPool>, vk_command_buffer: vk::CommandBuffer) -> Self {
        Self {
            pool: pool.clone(),
            vk_command_buffer,
        }
    }

    pub fn pool(&self) -> &Rc<CommandBufferPool> {
        &self.pool
    }

    pub unsafe fn handle(&self) -> vk::CommandBuffer {
        self.vk_command_buffer
    }

    pub fn begin(&self, flags: vk::CommandBufferUsageFlags) -> () {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .begin_command_buffer(
                    self.vk_command_buffer,
                    &vk::CommandBufferBeginInfo {
                        s_type: vk::StructureType::COMMAND_BUFFER_BEGIN_INFO,
                        p_next: std::ptr::null(),
                        flags,
                        p_inheritance_info: std::ptr::null(),
                    },
                )
                .expect("failed to begin command buffer recording");
        }
    }

    pub fn begin_render_pass(
        &self,
        render_pass: &Arc<RenderPass>,
        framebuffer: &Arc<Framebuffer>,
        render_area: vk::Rect2D,
        clear_values: Option<&[vk::ClearValue]>,
        contents: vk::SubpassContents,
    ) -> () {
        unsafe {
            let mut info = vk::RenderPassBeginInfo {
                s_type: vk::StructureType::RENDER_PASS_BEGIN_INFO,
                p_next: std::ptr::null(),
                render_pass: render_pass.get_vk_handle(),
                framebuffer: framebuffer.get_vk_handle(),
                render_area,
                clear_value_count: 0,
                p_clear_values: std::ptr::null(),
            };

            if let Some(clear_values) = clear_values {
                info.clear_value_count = clear_values.len().try_into().unwrap();
                info.p_clear_values = clear_values.as_ptr();
            }

            self.pool.device.get_ash_handle().cmd_begin_render_pass(
                self.vk_command_buffer,
                &info,
                contents,
            );
        }
    }

    pub fn bind_pipeline(
        &self,
        pipeline: &Arc<impl Pipeline + HasRawVkHandle<vk::Pipeline>>,
    ) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_bind_pipeline(
                self.vk_command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.get_vk_handle(),
            );
        }
    }

    pub fn set_viewport(&self, first_viewport: u32, viewports: &[vk::Viewport]) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_set_viewport(
                self.vk_command_buffer,
                first_viewport,
                viewports,
            );
        }
    }

    pub fn set_scissor(&self, first_scissor: u32, scissors: &[vk::Rect2D]) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_set_scissor(
                self.vk_command_buffer,
                first_scissor,
                scissors,
            );
        }
    }

    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_draw(
                self.vk_command_buffer,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            );
        }
    }

    pub fn end_render_pass(&self) -> () {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .cmd_end_render_pass(self.vk_command_buffer);
        }
    }

    pub fn end(&self) -> () {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .end_command_buffer(self.vk_command_buffer)
                .expect("failed to end command buffer recording");
        }
    }

    pub fn reset(&self) -> () {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .reset_command_buffer(self.vk_command_buffer, vk::CommandBufferResetFlags::empty())
                .expect("failed to reset command buffer");
        }
    }
}

impl HasRawVkHandle<vk::CommandBuffer> for CommandBuffer {
    unsafe fn get_vk_handle(&self) -> vk::CommandBuffer {
        self.vk_command_buffer
    }
}
