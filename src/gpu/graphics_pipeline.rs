use super::{Device, HasRawAshHandle, HasRawVkHandle, PipelineLayout, RenderPass, ShaderModule};
use ash::vk;
use std::sync::Arc;

pub struct GraphicsPipeline {
    device: Arc<Device>,
    vk_pipeline: vk::Pipeline,
}

pub trait Pipeline {
    fn bind_point(&self) -> vk::PipelineBindPoint;
}

impl GraphicsPipeline {
    pub fn new(
        device: &Arc<Device>,
        shader_modules: &Vec<Arc<ShaderModule>>,
        vertex_bindings: Option<&[vk::VertexInputBindingDescription]>,
        vertex_attributes: Option<&[vk::VertexInputAttributeDescription]>,
        dynamic_states: &Vec<vk::DynamicState>,
        topology: vk::PrimitiveTopology,
        primitive_restart: bool,
        _viewports: Option<&Vec<vk::Viewport>>,
        _scissors: Option<&Vec<vk::Rect2D>>,
        pipeline_layout: &Arc<PipelineLayout>,
        render_pass: &Arc<RenderPass>,
    ) -> Arc<GraphicsPipeline> {
        let mut create_info = unsafe {
            vk::GraphicsPipelineCreateInfo {
                s_type: vk::StructureType::GRAPHICS_PIPELINE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineCreateFlags::empty(),
                stage_count: 0,
                p_stages: std::ptr::null(),
                p_vertex_input_state: std::ptr::null(),
                p_input_assembly_state: std::ptr::null(),
                p_tessellation_state: std::ptr::null(),
                p_viewport_state: std::ptr::null(),
                p_rasterization_state: std::ptr::null(),
                p_multisample_state: std::ptr::null(),
                p_depth_stencil_state: std::ptr::null(),
                p_color_blend_state: std::ptr::null(),
                p_dynamic_state: std::ptr::null(),
                layout: pipeline_layout.get_vk_handle(),
                render_pass: render_pass.get_vk_handle(),
                subpass: 0,
                base_pipeline_handle: vk::Pipeline::null(),
                base_pipeline_index: -1,
            }
        };

        // ~~~~

        let mut shader_stage_create_infos = vec![];
        for shader_module in shader_modules {
            shader_stage_create_infos.push(*shader_module.pipeline_shader_stage_create_info());
        }
        create_info.stage_count = shader_stage_create_infos.len().try_into().unwrap();
        create_info.p_stages = shader_stage_create_infos.as_ptr();

        let mut _vertex_input_state_create_info = None;
        if !dynamic_states.contains(&vk::DynamicState::VERTEX_INPUT_EXT) {
            let mut handle = Box::new(vk::PipelineVertexInputStateCreateInfo {
                s_type: vk::StructureType::PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineVertexInputStateCreateFlags::empty(),
                vertex_binding_description_count: 0,
                p_vertex_binding_descriptions: std::ptr::null(),
                vertex_attribute_description_count: 0,
                p_vertex_attribute_descriptions: std::ptr::null(),
            });

            if let Some(bindings) = vertex_bindings {
                handle.vertex_binding_description_count = bindings.len().try_into().unwrap();
                handle.p_vertex_binding_descriptions = bindings.as_ptr();
            }

            if let Some(attributes) = vertex_attributes {
                handle.vertex_attribute_description_count = attributes.len().try_into().unwrap();
                handle.p_vertex_attribute_descriptions = attributes.as_ptr();
            }

            let ptr = &*handle as *const _;
            _vertex_input_state_create_info = Some(handle);
            create_info.p_vertex_input_state = ptr;
        }

        // TODO: Dynamic state
        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology,
            primitive_restart_enable: if primitive_restart {
                vk::TRUE
            } else {
                vk::FALSE
            },
        };
        create_info.p_input_assembly_state = &input_assembly_state_create_info;

        // TODO: Dynamic state
        let tessellation_state_create_info = vk::PipelineTessellationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_TESSELLATION_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineTessellationStateCreateFlags::empty(),
            patch_control_points: 0,
        };
        create_info.p_tessellation_state = &tessellation_state_create_info;

        // TODO: Really need to do some more complex logix to handle all the
        // edge cases here...
        //
        // TODO: Assuming that viewport/scissor is *always* provided at draw time
        let v_count = dynamic_states.contains(&vk::DynamicState::VIEWPORT_WITH_COUNT);
        let s_count = dynamic_states.contains(&vk::DynamicState::SCISSOR_WITH_COUNT);
        let mut _viewport_state_create_info = None;
        if !(v_count && s_count) {
            let handle = Box::new(vk::PipelineViewportStateCreateInfo {
                s_type: vk::StructureType::PIPELINE_VIEWPORT_STATE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineViewportStateCreateFlags::empty(),
                viewport_count: 1,
                p_viewports: std::ptr::null(),
                scissor_count: 1,
                p_scissors: std::ptr::null(),
            });
            let ptr = &*handle as *const _;
            _viewport_state_create_info = Some(handle);
            create_info.p_viewport_state = ptr;
        }

        // if let Some(viewports) = viewports {
        //     viewport_state_create_info.viewport_count = viewports.len().try_into().unwrap();
        //     viewport_state_create_info.p_viewports = viewports.as_ptr();
        // }

        // if let Some(scissors) = scissors {
        //     viewport_state_create_info.scissor_count = scissors.len().try_into().unwrap();
        //     viewport_state_create_info.p_scissors = scissors.as_ptr();
        // }

        // TODO: Hardcoded for now
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: vk::FALSE,
            rasterizer_discard_enable: vk::FALSE,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: vk::FALSE,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        create_info.p_rasterization_state = &rasterization_state_create_info;

        // TODO: Hardcoded for now
        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: vk::FALSE,
            min_sample_shading: 1.0,
            p_sample_mask: std::ptr::null(),
            alpha_to_coverage_enable: vk::FALSE,
            alpha_to_one_enable: vk::FALSE,
        };
        create_info.p_multisample_state = &multisample_state_create_info;

        // TODO: Hardcoded null for depth/stencil for now

        // XXX
        // TODO: Depends on number of attachements
        assert!(render_pass.attachment_count() == 1);
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        };

        // XXX
        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op_enable: vk::FALSE,
            logic_op: vk::LogicOp::COPY,
            attachment_count: 1,
            p_attachments: &color_blend_attachment,
            blend_constants: [0.0, 0.0, 0.0, 0.0],
        };
        create_info.p_color_blend_state = &color_blend_state_create_info;

        // XXX
        assert!(dynamic_states.contains(&vk::DynamicState::VIEWPORT));
        assert!(dynamic_states.contains(&vk::DynamicState::SCISSOR));
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo {
            s_type: vk::StructureType::PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_state_count: dynamic_states.len().try_into().unwrap(),
            p_dynamic_states: dynamic_states.as_ptr(),
        };
        create_info.p_dynamic_state = &dynamic_state_create_info;

        let create_infos = [create_info];

        // TODO: Vulkan clearly wants us to be creating pipelines in batches.
        // Need a builder / loader pattern for that
        let vk_pipeline = unsafe {
            let pipelines = device
                .get_ash_handle()
                .create_graphics_pipelines(vk::PipelineCache::null(), &create_infos, None)
                .expect("failed to create graphics pipeline(s)");
            pipelines[0]
        };

        Arc::new(GraphicsPipeline {
            device: device.clone(),
            vk_pipeline,
        })
    }
}

impl Pipeline for GraphicsPipeline {
    fn bind_point(&self) -> vk::PipelineBindPoint {
        vk::PipelineBindPoint::GRAPHICS
    }
}

impl HasRawVkHandle<vk::Pipeline> for GraphicsPipeline {
    unsafe fn get_vk_handle(&self) -> vk::Pipeline {
        self.vk_pipeline
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_pipeline(self.vk_pipeline, None);
        }
    }
}
