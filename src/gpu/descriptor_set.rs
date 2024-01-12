use super::{Buffer, Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::{cell::OnceCell, collections::HashMap, sync::Arc};

pub struct DescriptorPool {
    device: Arc<Device>,
    vk_descriptor_pool: vk::DescriptorPool,
}

impl DescriptorPool {
    pub fn new(
        device: Arc<Device>,
        flags: vk::DescriptorPoolCreateFlags,
        max_sets: u32,
        set_types: &[(vk::DescriptorType, u32)],
    ) -> Self {
        let vk_pool_sizes = set_types
            .iter()
            .map(|(ty, descriptor_count)| vk::DescriptorPoolSize {
                ty: *ty,
                descriptor_count: *descriptor_count,
            })
            .collect::<Box<[_]>>();

        let info = vk::DescriptorPoolCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_POOL_CREATE_INFO,
            p_next: std::ptr::null(),
            flags,
            max_sets,
            pool_size_count: vk_pool_sizes.len().try_into().unwrap(),
            p_pool_sizes: vk_pool_sizes.as_ptr(),
        };

        let vk_descriptor_pool = unsafe {
            device
                .get_ash_handle()
                .create_descriptor_pool(&info, None)
                .expect("failed to create descriptor pool")
        };

        Self {
            device,
            vk_descriptor_pool,
        }
    }

    pub fn allocate(&self, set_layouts: &[&DescriptorSetLayout]) -> Box<[DescriptorSet]> {
        unsafe {
            let vk_set_layouts = set_layouts
                .iter()
                .map(|x| x.get_vk_handle())
                .collect::<Box<_>>();

            let info = vk::DescriptorSetAllocateInfo {
                s_type: vk::StructureType::DESCRIPTOR_SET_ALLOCATE_INFO,
                p_next: std::ptr::null(),
                descriptor_pool: self.vk_descriptor_pool,
                descriptor_set_count: vk_set_layouts.len().try_into().unwrap(),
                p_set_layouts: vk_set_layouts.as_ptr(),
            };

            self.device
                .get_ash_handle()
                .allocate_descriptor_sets(&info)
                .expect("failed to allocate descriptor sets")
                .into_iter()
                .map(|x| DescriptorSet::new(self.device.clone(), x))
                .collect()
        }
    }
}

impl HasRawVkHandle<vk::DescriptorPool> for DescriptorPool {
    unsafe fn get_vk_handle(&self) -> vk::DescriptorPool {
        self.vk_descriptor_pool
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_descriptor_pool(self.vk_descriptor_pool, None)
        }
    }
}

pub struct DescriptorSet {
    device: Arc<Device>,
    vk_descriptor_set: vk::DescriptorSet,
}

impl DescriptorSet {
    pub fn new(device: Arc<Device>, vk_descriptor_set: vk::DescriptorSet) -> Self {
        Self {
            device,
            vk_descriptor_set,
        }
    }

    pub fn write_buffer(
        &self,
        buffer: &Buffer,
        offset: u64,
        range: u64,
        binding: u32,
        element: u32,
        ty: vk::DescriptorType,
    ) {
        unsafe {
            let buffer_info = vk::DescriptorBufferInfo {
                buffer: buffer.get_vk_handle(),
                offset,
                range,
            };

            let write = vk::WriteDescriptorSet {
                s_type: vk::StructureType::WRITE_DESCRIPTOR_SET,
                p_next: std::ptr::null(),
                dst_set: self.get_vk_handle(),
                dst_binding: binding,
                dst_array_element: element,
                descriptor_count: 1,
                descriptor_type: ty,
                p_image_info: std::ptr::null(),
                p_buffer_info: &buffer_info,
                p_texel_buffer_view: std::ptr::null(),
            };

            self.device
                .get_ash_handle()
                .update_descriptor_sets(&[write], &[]);
        }
        //
    }
}

impl HasRawVkHandle<vk::DescriptorSet> for DescriptorSet {
    unsafe fn get_vk_handle(&self) -> vk::DescriptorSet {
        self.vk_descriptor_set
    }
}

pub struct DescriptorSetLayout {
    device: Arc<Device>,
    vk_descriptor_set_layout: vk::DescriptorSetLayout,
}

impl DescriptorSetLayout {
    pub fn builder() -> DescriptorSetLayoutBuilder {
        DescriptorSetLayoutBuilder::new()
    }

    pub fn new(
        device: Arc<Device>,
        vk_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Arc<Self> {
        Arc::new(Self {
            device,
            vk_descriptor_set_layout,
        })
    }
}

impl HasRawVkHandle<vk::DescriptorSetLayout> for DescriptorSetLayout {
    unsafe fn get_vk_handle(&self) -> vk::DescriptorSetLayout {
        self.vk_descriptor_set_layout
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_descriptor_set_layout(self.vk_descriptor_set_layout, None)
        };
    }
}

pub struct DescriptorSetLayoutBuilder {
    id: OnceCell<usize>,
    next_id: usize,
}

impl DescriptorSetLayoutBuilder {
    pub fn new() -> Self {
        Self {
            id: OnceCell::new(),
            next_id: 0,
        }
    }

    pub fn id(&self) -> usize {
        *self
            .id
            .get_or_init(|| (self as *const DescriptorSetLayoutBuilder) as usize)
    }

    fn _get_next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn binding(&mut self) -> DescriptorSetLayoutBindingBuilder {
        DescriptorSetLayoutBindingBuilder::new(self._get_next_id(), self.id())
    }

    pub fn build(
        &self,
        device: Arc<Device>,
        flags: vk::DescriptorSetLayoutCreateFlags,
        bindings: &[DescriptorSetLayoutBindingBuilder],
    ) -> Arc<DescriptorSetLayout> {
        let mut layout_binding_indices = HashMap::<usize, u32>::new();

        for (i, x) in bindings.iter().enumerate() {
            let binding: u32 = i.try_into().unwrap();
            layout_binding_indices.insert(x.id, binding);
        }

        let vk_bindings = bindings
            .iter()
            .map(|x| x.get_descriptor_set_layout_binding(&layout_binding_indices))
            .collect::<Vec<_>>();

        let info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: std::ptr::null(),
            flags,
            binding_count: vk_bindings.len().try_into().unwrap(),
            p_bindings: vk_bindings.as_ptr(),
        };

        let vk_descriptor_set_layout = unsafe {
            device
                .get_ash_handle()
                .create_descriptor_set_layout(&info, None)
                .expect("failed to create descriptor set layout")
        };

        DescriptorSetLayout::new(device, vk_descriptor_set_layout)
    }
}

pub struct DescriptorSetLayoutBindingBuilder {
    id: usize,
    parent_id: usize,
    count_and_type: Option<(u32, vk::DescriptorType)>,
    shader_stage: Option<vk::ShaderStageFlags>,
}

impl DescriptorSetLayoutBindingBuilder {
    pub fn new(id: usize, parent_id: usize) -> Self {
        Self {
            id,
            parent_id,
            count_and_type: None,
            shader_stage: None,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn parent_id(&self) -> usize {
        self.parent_id
    }

    pub fn descriptor(mut self, count: u32, ty: vk::DescriptorType) -> Self {
        self.count_and_type = Some((count, ty));
        self
    }

    pub fn stage(mut self, shader_stage: vk::ShaderStageFlags) -> Self {
        self.shader_stage = Some(shader_stage);
        self
    }

    pub fn get_descriptor_set_layout_binding(
        &self,
        layout_binding_indices: &HashMap<usize, u32>,
    ) -> vk::DescriptorSetLayoutBinding {
        let (descriptor_count, descriptor_type) = self.count_and_type.unwrap();
        let stage_flags = self.shader_stage.unwrap();

        vk::DescriptorSetLayoutBinding {
            binding: *layout_binding_indices.get(&self.id).unwrap(),
            descriptor_type,
            descriptor_count,
            stage_flags,
            p_immutable_samplers: std::ptr::null(),
        }
    }
}
