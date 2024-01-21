use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use core::panic;
use std::{cell::OnceCell, collections::HashMap, sync::Arc};

pub struct RenderPass {
    device: Arc<Device>,
    vk_render_pass: vk::RenderPass,
    attachment_count: u32,
}

impl RenderPass {
    pub fn new(
        device: Arc<Device>,
        vk_render_pass: vk::RenderPass,
        attachment_count: u32,
    ) -> Arc<RenderPass> {
        Arc::new(RenderPass {
            device,
            vk_render_pass,
            attachment_count,
        })
    }

    pub fn builder() -> RenderPassBuilder {
        RenderPassBuilder::new()
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn attachment_count(&self) -> u32 {
        self.attachment_count
    }
}

impl HasRawVkHandle<vk::RenderPass> for RenderPass {
    unsafe fn get_vk_handle(&self) -> vk::RenderPass {
        self.vk_render_pass
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_render_pass(self.vk_render_pass, None);
        }
    }
}

pub struct RenderPassBuilder {
    id: OnceCell<usize>,
    next_id: usize,
}

pub struct RenderPassConfig<'t> {
    pub device: &'t Arc<Device>,
    pub attachments: Vec<AttachmentBuilder>,
    pub subpasses: Vec<SubpassBuilder>,
    pub dependencies: Option<Vec<DependencyBuilder>>,
}

impl RenderPassBuilder {
    pub fn new() -> RenderPassBuilder {
        RenderPassBuilder {
            id: OnceCell::new(),
            next_id: 0,
        }
    }

    pub fn id(&self) -> usize {
        *self
            .id
            .get_or_init(|| (self as *const RenderPassBuilder) as usize)
    }

    fn _get_next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn attachment(&mut self) -> AttachmentBuilder {
        AttachmentBuilder::new(self._get_next_id(), self.id())
    }

    pub fn subpass(&mut self) -> SubpassBuilder {
        SubpassBuilder::new(self._get_next_id(), self.id())
    }

    pub fn dependency(&mut self) -> DependencyBuilder {
        DependencyBuilder::new(self._get_next_id(), self.id())
    }

    pub fn build(self, config: RenderPassConfig) -> Arc<RenderPass> {
        // Collect attachments
        let mut attachment_indices: HashMap<usize, u32> = HashMap::new();
        let mut attachment_descriptions: Vec<vk::AttachmentDescription> = vec![];
        attachment_descriptions.reserve(config.attachments.len());
        for (index, attachment) in config.attachments.iter().enumerate() {
            assert!(attachment.parent_id() == self.id());
            let res = attachment_indices.insert(attachment.id(), index.try_into().unwrap());
            if res.is_some() {
                panic!("duplicate attachment builder id");
            }
            attachment_descriptions.push(attachment.attachment_description());
        }

        // Collect the subpass builders
        let mut subpass_indices: HashMap<usize, u32> = HashMap::new();
        let mut subpass_descriptions: Vec<vk::SubpassDescription> = vec![];
        subpass_descriptions.reserve(config.subpasses.len());
        for (index, subpass) in config.subpasses.iter().enumerate() {
            assert!(subpass.parent_id() == self.id());
            let res = subpass_indices.insert(subpass.id(), index.try_into().unwrap());
            if res.is_some() {
                panic!("duplicate subpass builder id");
            }

            let subpass_description =
                unsafe { subpass.get_subpass_description(&attachment_indices) };

            subpass_descriptions.push(subpass_description);
        }

        let mut dependency_indices: HashMap<usize, u32> = HashMap::new();
        let mut vk_dependencies: Vec<vk::SubpassDependency> = vec![];
        if let Some(dependencies) = config.dependencies {
            for (index, dependency) in dependencies.iter().enumerate() {
                assert!(dependency.parent_id() == self.id());
                let res = dependency_indices.insert(dependency.id(), index.try_into().unwrap());
                if res.is_some() {
                    panic!("duplicate dependency builder id");
                }
                vk_dependencies.push(dependency.get_subpass_dependency(&subpass_indices));
            }
        }

        let mut render_pass_create_info = vk::RenderPassCreateInfo {
            s_type: vk::StructureType::RENDER_PASS_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::RenderPassCreateFlags::empty(),
            attachment_count: 0,
            p_attachments: std::ptr::null(),
            subpass_count: 0,
            p_subpasses: std::ptr::null(),
            dependency_count: 0,
            p_dependencies: std::ptr::null(),
        };

        if attachment_descriptions.len() > 0 {
            render_pass_create_info.attachment_count =
                attachment_descriptions.len().try_into().unwrap();
            render_pass_create_info.p_attachments = attachment_descriptions.as_ptr();
        }

        if subpass_descriptions.len() > 0 {
            render_pass_create_info.subpass_count = subpass_descriptions.len().try_into().unwrap();
            render_pass_create_info.p_subpasses = subpass_descriptions.as_ptr();
        }

        if vk_dependencies.len() > 0 {
            render_pass_create_info.dependency_count = vk_dependencies.len().try_into().unwrap();
            render_pass_create_info.p_dependencies = vk_dependencies.as_ptr();
        }

        let vk_render_pass = unsafe {
            config
                .device
                .get_ash_handle()
                .create_render_pass(&render_pass_create_info, None)
                .expect("failed to create render pass")
        };

        RenderPass::new(
            config.device.clone(),
            vk_render_pass,
            config.attachments.len().try_into().unwrap(),
        )
    }
}

pub struct AttachmentBuilder {
    id: usize,
    parent_id: usize,
    vk_description: vk::AttachmentDescription,
}

impl AttachmentBuilder {
    pub fn new(id: usize, parent_id: usize) -> AttachmentBuilder {
        AttachmentBuilder {
            id,
            parent_id,
            vk_description: vk::AttachmentDescription::default(),
        }
    }

    pub fn parent_id(&self) -> usize {
        self.parent_id
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn format(mut self, format: vk::Format) -> AttachmentBuilder {
        self.vk_description.format = format;
        self
    }

    pub fn samples(mut self, samples: vk::SampleCountFlags) -> AttachmentBuilder {
        self.vk_description.samples = samples;
        self
    }

    pub fn load_op(mut self, load_op: vk::AttachmentLoadOp) -> AttachmentBuilder {
        self.vk_description.load_op = load_op;
        self
    }

    pub fn store_op(mut self, store_op: vk::AttachmentStoreOp) -> AttachmentBuilder {
        self.vk_description.store_op = store_op;
        self
    }

    pub fn stencil_load_op(mut self, stencil_load_op: vk::AttachmentLoadOp) -> AttachmentBuilder {
        self.vk_description.load_op = stencil_load_op;
        self
    }

    pub fn stencil_store_op(
        mut self,
        stencil_store_op: vk::AttachmentStoreOp,
    ) -> AttachmentBuilder {
        self.vk_description.stencil_store_op = stencil_store_op;
        self
    }

    pub fn initial_layout(mut self, initial_layout: vk::ImageLayout) -> AttachmentBuilder {
        self.vk_description.initial_layout = initial_layout;
        self
    }

    pub fn final_layout(mut self, final_layout: vk::ImageLayout) -> AttachmentBuilder {
        self.vk_description.final_layout = final_layout;
        self
    }

    pub fn attachment_description(&self) -> vk::AttachmentDescription {
        self.vk_description
    }
}

pub struct SubpassBuilder {
    id: usize,
    parent_id: usize,
    input: Vec<(usize, vk::ImageLayout)>,
    color: Vec<(usize, vk::ImageLayout)>,
    resolve: Vec<(usize, vk::ImageLayout)>,
    depth_stencil: Option<(usize, vk::ImageLayout)>,
    preserve: Vec<usize>,
    description_state: OnceCell<SubpassDescriptionState>,
}

struct SubpassDescriptionState {
    subpass_description: vk::SubpassDescription,
    input_attachments: Box<Vec<vk::AttachmentReference>>,
    color_attachments: Box<Vec<vk::AttachmentReference>>,
    resolve_attachments: Box<Vec<vk::AttachmentReference>>,
    depth_stencil_attachment: Box<vk::AttachmentReference>,
    preserve_attachments: Box<Vec<u32>>,
}

impl SubpassBuilder {
    pub fn new(id: usize, parent_id: usize) -> SubpassBuilder {
        SubpassBuilder {
            id,
            parent_id,
            input: vec![],
            color: vec![],
            resolve: vec![],
            depth_stencil: None,
            preserve: vec![],
            description_state: OnceCell::new(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn parent_id(&self) -> usize {
        self.parent_id
    }

    pub fn input(
        mut self,
        attachment: &AttachmentBuilder,
        layout: vk::ImageLayout,
    ) -> SubpassBuilder {
        self.input.push((attachment.id(), layout));
        self
    }

    pub fn color(
        mut self,
        attachment: &AttachmentBuilder,
        layout: vk::ImageLayout,
    ) -> SubpassBuilder {
        self.color.push((attachment.id(), layout));
        self
    }

    pub fn resolve(
        mut self,
        attachment: &AttachmentBuilder,
        layout: vk::ImageLayout,
    ) -> SubpassBuilder {
        self.resolve.push((attachment.id(), layout));
        self
    }

    pub fn depth_stencil(
        mut self,
        attachment: &AttachmentBuilder,
        layout: vk::ImageLayout,
    ) -> SubpassBuilder {
        self.depth_stencil = Some((attachment.id(), layout));
        self
    }

    pub fn preserve(mut self, attachment: &AttachmentBuilder) -> SubpassBuilder {
        self.preserve.push(attachment.id());
        self
    }

    pub unsafe fn get_subpass_description(
        &self,
        attachment_indices: &HashMap<usize, u32>,
    ) -> vk::SubpassDescription {
        let state = self.description_state.get_or_init(|| {
            let mut state = SubpassDescriptionState {
                subpass_description: vk::SubpassDescription {
                    flags: vk::SubpassDescriptionFlags::empty(),
                    pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
                    input_attachment_count: 0,
                    p_input_attachments: std::ptr::null(),
                    color_attachment_count: 0,
                    p_color_attachments: std::ptr::null(),
                    p_resolve_attachments: std::ptr::null(),
                    p_depth_stencil_attachment: std::ptr::null(),
                    preserve_attachment_count: 0,
                    p_preserve_attachments: std::ptr::null(),
                },
                input_attachments: Box::new(vec![]),
                color_attachments: Box::new(vec![]),
                resolve_attachments: Box::new(vec![]),
                depth_stencil_attachment: Box::new(vk::AttachmentReference::default()),
                preserve_attachments: Box::new(vec![]),
            };

            state.input_attachments.reserve(self.input.len());
            state.color_attachments.reserve(self.color.len());
            state.resolve_attachments.reserve(self.resolve.len());
            state.preserve_attachments.reserve(self.preserve.len());

            if self.input.len() > 0 {
                for (id, layout) in &self.input {
                    state.input_attachments.push(vk::AttachmentReference {
                        attachment: *attachment_indices.get(id).unwrap(),
                        layout: *layout,
                    });
                }
                state.subpass_description.input_attachment_count =
                    self.input.len().try_into().unwrap();
                state.subpass_description.p_input_attachments = state.input_attachments.as_ptr();
            }

            if self.color.len() > 0 {
                for (id, layout) in &self.color {
                    state.color_attachments.push(vk::AttachmentReference {
                        attachment: *attachment_indices.get(id).unwrap(),
                        layout: *layout,
                    });
                }
                state.subpass_description.color_attachment_count =
                    self.color.len().try_into().unwrap();
                state.subpass_description.p_color_attachments = state.color_attachments.as_ptr();
            }

            if self.resolve.len() > 0 {
                if self.resolve.len() != self.color.len() {
                    panic!(
                    "number of subpass resolve attachments must equal number of color attachments"
                );
                }

                for (id, layout) in &self.resolve {
                    state.input_attachments.push(vk::AttachmentReference {
                        attachment: *attachment_indices.get(id).unwrap(),
                        layout: *layout,
                    });
                }
                state.subpass_description.p_resolve_attachments =
                    state.resolve_attachments.as_ptr();
            }

            if let Some((id, layout)) = &self.depth_stencil {
                state.depth_stencil_attachment.attachment = *attachment_indices.get(id).unwrap();
                state.depth_stencil_attachment.layout = *layout;
                state.subpass_description.p_depth_stencil_attachment =
                    state.depth_stencil_attachment.as_ref();
            }

            if self.preserve.len() > 0 {
                for id in &self.preserve {
                    state
                        .preserve_attachments
                        .push(*attachment_indices.get(id).unwrap());
                }
                state.subpass_description.preserve_attachment_count =
                    self.color.len().try_into().unwrap();
                state.subpass_description.p_preserve_attachments =
                    state.preserve_attachments.as_ptr();
            }

            state
        });

        state.subpass_description
    }
}

pub struct DependencyBuilder {
    id: usize,
    parent_id: usize,
    src_subpass: Option<SubpassRef>,
    dst_subpass: Option<SubpassRef>,
    src_stage_mask: Option<vk::PipelineStageFlags>,
    dst_stage_mask: Option<vk::PipelineStageFlags>,
    src_access_mask: Option<vk::AccessFlags>,
    dst_access_mask: Option<vk::AccessFlags>,
    flags: Option<vk::DependencyFlags>,
}

enum SubpassRef {
    Subpass(usize),
    External,
}

impl DependencyBuilder {
    pub fn new(id: usize, parent_id: usize) -> DependencyBuilder {
        Self {
            id,
            parent_id,
            src_subpass: None,
            dst_subpass: None,
            src_stage_mask: None,
            dst_stage_mask: None,
            src_access_mask: None,
            dst_access_mask: None,
            flags: None,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn parent_id(&self) -> usize {
        self.parent_id
    }

    pub fn src(mut self, subpass: &SubpassBuilder) -> Self {
        self.src_subpass = Some(SubpassRef::Subpass(subpass.id()));
        self
    }

    pub fn src_external(mut self) -> Self {
        self.src_subpass = Some(SubpassRef::External);
        self
    }

    pub fn dst(mut self, subpass: &SubpassBuilder) -> Self {
        self.dst_subpass = Some(SubpassRef::Subpass(subpass.id()));
        self
    }

    pub fn dst_external(mut self) -> Self {
        self.dst_subpass = Some(SubpassRef::External);
        self
    }

    pub fn src_stage_mask(mut self, mask: vk::PipelineStageFlags) -> Self {
        self.src_stage_mask = Some(mask);
        self
    }

    pub fn dst_stage_mask(mut self, mask: vk::PipelineStageFlags) -> Self {
        self.dst_stage_mask = Some(mask);
        self
    }

    pub fn src_access_mask(mut self, mask: vk::AccessFlags) -> Self {
        self.src_access_mask = Some(mask);
        self
    }

    pub fn dst_access_mask(mut self, mask: vk::AccessFlags) -> Self {
        self.dst_access_mask = Some(mask);
        self
    }

    pub fn flags(mut self, mask: vk::DependencyFlags) -> Self {
        self.flags = Some(mask);
        self
    }

    pub fn get_subpass_dependency(
        &self,
        subpass_indices: &HashMap<usize, u32>,
    ) -> vk::SubpassDependency {
        let mut subpass_dependency = vk::SubpassDependency::default();

        if let Some(src_subpass_ref) = &self.src_subpass {
            match src_subpass_ref {
                SubpassRef::Subpass(id) => {
                    subpass_dependency.src_subpass = *subpass_indices.get(id).unwrap();
                }
                SubpassRef::External => {
                    subpass_dependency.src_subpass = vk::SUBPASS_EXTERNAL;
                }
            }
        }

        if let Some(dst_subpass_ref) = &self.dst_subpass {
            match dst_subpass_ref {
                SubpassRef::Subpass(id) => {
                    subpass_dependency.dst_subpass = *subpass_indices.get(id).unwrap();
                }
                SubpassRef::External => {
                    subpass_dependency.dst_subpass = vk::SUBPASS_EXTERNAL;
                }
            }
        }

        if let Some(src_stage_mask) = self.src_stage_mask {
            subpass_dependency.src_stage_mask = src_stage_mask;
        }

        if let Some(dst_stage_mask) = self.dst_stage_mask {
            subpass_dependency.dst_stage_mask = dst_stage_mask;
        }

        if let Some(src_access_mask) = self.src_access_mask {
            subpass_dependency.src_access_mask = src_access_mask;
        }

        if let Some(dst_access_mask) = self.dst_access_mask {
            subpass_dependency.dst_access_mask = dst_access_mask;
        }

        if let Some(flags) = self.flags {
            subpass_dependency.dependency_flags = flags;
        }

        subpass_dependency
    }
}
