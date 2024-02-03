use super::{
    Buffer, DescriptorSet, Device, Framebuffer, Image, Pipeline, PipelineLayout, QueueFamily,
    RenderPass,
};
use super::{HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::rc::Rc;
use std::sync::Arc;

pub struct CommandPool {
    device: Arc<Device>,
    vk_command_pool: vk::CommandPool,
}

impl CommandPool {
    pub fn new(
        device: Arc<Device>,
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
            device: device,
            vk_command_pool,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub unsafe fn handle(&self) -> vk::CommandPool {
        self.vk_command_pool
    }

    pub fn allocate_one(self: &Rc<CommandPool>, level: vk::CommandBufferLevel) -> CommandBuffer {
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

        CommandBuffer::new(self.clone(), vk_command_buffer)
    }
}

impl HasRawVkHandle<vk::CommandPool> for CommandPool {
    unsafe fn get_vk_handle(&self) -> vk::CommandPool {
        self.vk_command_pool
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_command_pool(self.vk_command_pool, None);
        }
    }
}

pub struct CommandBuffer {
    pool: Rc<CommandPool>,
    vk_command_buffer: vk::CommandBuffer,
}

impl CommandBuffer {
    pub fn new(pool: Rc<CommandPool>, vk_command_buffer: vk::CommandBuffer) -> Self {
        Self {
            pool,
            vk_command_buffer,
        }
    }

    pub fn pool(&self) -> &Rc<CommandPool> {
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

    pub fn clear_color_image(
        &self,
        image: &Image,
        clear_value: vk::ClearColorValue,
        clear_range: &[vk::ImageSubresourceRange],
    ) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_clear_color_image(
                self.vk_command_buffer,
                image.get_vk_handle(),
                vk::ImageLayout::GENERAL,
                &clear_value,
                clear_range,
            )
        }
    }

    pub fn begin_render_pass(
        &self,
        render_pass: &RenderPass,
        framebuffer: &Framebuffer,
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

    pub fn begin_rendering(
        &self,
        flags: vk::RenderingFlags,
        render_area: vk::Rect2D,
        layer_count: u32,
        view_mask: u32,
        color_attachments: Option<&[vk::RenderingAttachmentInfo]>,
        depth_attachment: Option<vk::RenderingAttachmentInfo>,
        stencil_attachment: Option<vk::RenderingAttachmentInfo>,
    ) -> () {
        let mut info = vk::RenderingInfo {
            s_type: vk::StructureType::RENDERING_INFO,
            p_next: std::ptr::null(),
            flags,
            render_area,
            layer_count,
            view_mask,
            color_attachment_count: 0,
            p_color_attachments: std::ptr::null(),
            p_depth_attachment: std::ptr::null(),
            p_stencil_attachment: std::ptr::null(),
        };

        unsafe {
            if let Some(color) = color_attachments {
                info.color_attachment_count = color.len().try_into().unwrap();
                info.p_color_attachments = color.as_ptr();
            }

            if let Some(depth) = depth_attachment {
                info.p_depth_attachment = &depth;
            }

            if let Some(stencil) = stencil_attachment {
                info.p_stencil_attachment = &stencil;
            }

            self.pool
                .device
                .get_ash_handle()
                .cmd_begin_rendering(self.vk_command_buffer, &info);
        }
    }

    pub fn bind_pipeline<T>(&self, pipeline: &T) -> ()
    where
        T: Pipeline + HasRawVkHandle<vk::Pipeline>,
    {
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

    pub fn bind_index_buffer(
        &self,
        buffer: &Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_bind_index_buffer(
                self.vk_command_buffer,
                buffer.get_vk_handle(),
                offset,
                index_type,
            );
        }
    }

    // TODO: u64's should be vk::DeviceSize
    pub fn bind_vertex_buffers(&self, first_binding: u32, buffers: &[(&Buffer, u64)]) -> () {
        unsafe {
            let vk_buffer: Vec<_> = buffers.iter().map(|x| x.0.get_vk_handle()).collect();
            let vk_offsets: Vec<_> = buffers.iter().map(|x| x.1).collect();
            self.pool.device.get_ash_handle().cmd_bind_vertex_buffers(
                self.vk_command_buffer,
                first_binding,
                &vk_buffer,
                vk_offsets.as_slice(),
            );
        }
    }

    pub fn bind_descriptor_sets(
        &self,
        pipeline_bind_point: vk::PipelineBindPoint,
        layout: &PipelineLayout,
        first_set: u32,
        descriptor_sets: &[&DescriptorSet],
    ) {
        unsafe {
            let vk_descriptor_sets = descriptor_sets
                .iter()
                .map(|x| x.get_vk_handle())
                .collect::<Box<_>>();

            self.pool.device.get_ash_handle().cmd_bind_descriptor_sets(
                self.vk_command_buffer,
                pipeline_bind_point,
                layout.get_vk_handle(),
                first_set,
                &vk_descriptor_sets,
                &[],
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

    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_draw_indexed(
                self.vk_command_buffer,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
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

    pub fn end_rendering(&self) -> () {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .cmd_end_rendering(self.vk_command_buffer);
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

    pub fn transition_image(
        &self,
        image: &Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> () {
        let aspect_mask = match new_layout {
            vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL => vk::ImageAspectFlags::DEPTH,
            _ => vk::ImageAspectFlags::COLOR,
        };

        unsafe {
            let image_barrier = vk::ImageMemoryBarrier2 {
                s_type: vk::StructureType::IMAGE_MEMORY_BARRIER_2_KHR,
                p_next: std::ptr::null(),
                src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                src_access_mask: vk::AccessFlags2::MEMORY_WRITE,
                dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
                dst_access_mask: vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
                old_layout,
                new_layout,
                // src_queue_family_index: todo!(),
                // dst_queue_family_index: todo!(),
                image: image.get_vk_handle(),
                subresource_range: vk::ImageSubresourceRange {
                    aspect_mask,
                    base_mip_level: 0,
                    level_count: vk::REMAINING_MIP_LEVELS,
                    base_array_layer: 0,
                    layer_count: vk::REMAINING_ARRAY_LAYERS,
                },
                ..Default::default()
            };

            let dep_info = vk::DependencyInfo {
                s_type: vk::StructureType::DEPENDENCY_INFO,
                p_next: std::ptr::null(),
                dependency_flags: vk::DependencyFlags::empty(),
                memory_barrier_count: 0,
                p_memory_barriers: std::ptr::null(),
                buffer_memory_barrier_count: 0,
                p_buffer_memory_barriers: std::ptr::null(),
                image_memory_barrier_count: 1,
                p_image_memory_barriers: &image_barrier,
            };

            self.pool
                .device
                .get_ash_handle()
                .cmd_pipeline_barrier2(self.vk_command_buffer, &dep_info)
        }
    }

    pub fn blit_image(&self, blit_image_info: &vk::BlitImageInfo2) -> () {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .cmd_blit_image2(self.vk_command_buffer, blit_image_info)
        }
    }

    pub fn copy_buffer(&self, src: &Buffer, dst: &Buffer, regions: &[vk::BufferCopy]) -> () {
        unsafe {
            self.pool.device.get_ash_handle().cmd_copy_buffer(
                self.vk_command_buffer,
                src.get_vk_handle(),
                dst.get_vk_handle(),
                regions,
            );
        }
    }

    pub fn copy_buffer_to_image(&self, src: &Buffer, dst: &Image) -> () {
        let extent = dst.extent();

        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
        };

        unsafe {
            self.pool.device.get_ash_handle().cmd_copy_buffer_to_image(
                self.vk_command_buffer,
                src.get_vk_handle(),
                dst.get_vk_handle(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            )
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

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.pool
                .device
                .get_ash_handle()
                .free_command_buffers(self.pool.vk_command_pool, &[self.vk_command_buffer]);
        }
    }
}
