use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use core::panic;
use std::{mem::size_of, os::raw::c_void, sync::Arc};

pub struct Buffer {
    device: Arc<Device>,
    vk_buffer: vk::Buffer,
    vk_memory: Option<vk::DeviceMemory>,
}

impl Buffer {
    pub fn new(device: Arc<Device>, size: usize, usage: vk::BufferUsageFlags) -> Self {
        let create_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size.try_into().unwrap(),
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };

        let vk_buffer = unsafe {
            device
                .get_ash_handle()
                .create_buffer(&create_info, None)
                .expect("failed to create buffer")
        };

        Self {
            device: device,
            vk_buffer,
            vk_memory: None,
        }
    }

    pub fn get_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.device
                .get_ash_handle()
                .get_buffer_memory_requirements(self.vk_buffer)
        }
    }

    pub fn allocate(&mut self, properties: vk::MemoryPropertyFlags) -> () {
        let memory_requirements = self.get_memory_requirements();

        let memory_properties = self.device.physical_device().get_memory_properties();

        let mut memory_type_index: u32 = 0;
        let mut found_memory_type_index = false;

        while memory_type_index < memory_properties.memory_type_count {
            let i: usize = memory_type_index.try_into().unwrap();
            if memory_requirements.memory_type_bits & (1 << memory_type_index) > 0 {
                let memory_type = memory_properties.memory_types[i];
                if (memory_type.property_flags & properties) == properties {
                    found_memory_type_index = true;
                    break;
                }
            }
            memory_type_index += 1;
        }

        if !found_memory_type_index {
            panic!("no memory found supporting the requested properties");
        }

        let info = vk::MemoryAllocateInfo {
            s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
            p_next: std::ptr::null(),
            allocation_size: memory_requirements.size,
            memory_type_index,
        };

        self.vk_memory = unsafe {
            let ash_device = self.device.get_ash_handle();

            let vk_memory = ash_device
                .allocate_memory(&info, None)
                .expect("failed to allocate memory");

            ash_device
                .bind_buffer_memory(self.vk_buffer, vk_memory, 0)
                .expect("failed to bind device memory to buffer");

            Some(vk_memory)
        };
    }

    pub fn copy_nonoverlapping<T>(&self, src: &[T]) -> () {
        unsafe {
            let ash_device = self.device.get_ash_handle();

            let memory = self.vk_memory.unwrap();
            let offset: vk::DeviceSize = 0;
            let size = size_of::<T>() * src.len();
            let vk_size: vk::DeviceSize = size.try_into().unwrap();

            let dst = ash_device
                .map_memory(memory, offset, vk_size, vk::MemoryMapFlags::empty())
                .expect("failed to map memory");

            std::ptr::copy_nonoverlapping(src.as_ptr() as *const c_void, dst, size);

            ash_device.unmap_memory(memory);
        }
    }
}

impl HasRawVkHandle<vk::Buffer> for Buffer {
    unsafe fn get_vk_handle(&self) -> vk::Buffer {
        self.vk_buffer
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            let ash_device = self.device.get_ash_handle();

            ash_device.destroy_buffer(self.vk_buffer, None);

            if let Some(memory) = self.vk_memory {
                ash_device.free_memory(memory, None);
            }
        }
    }
}
