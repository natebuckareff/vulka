use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use std::{ffi::c_void, mem::size_of, sync::Arc};
use vma::Alloc;

pub struct Buffer {
    device: Arc<Device>,
    allocator: Arc<vma::Allocator>,
    vk_buffer: vk::Buffer,
    vma_allocation: vma::Allocation,
    vma_allocation_info: vma::AllocationInfo,
}

impl Buffer {
    pub fn new(
        device: Arc<Device>,
        allocator: Arc<vma::Allocator>,
        size: usize,
        buffer_usage: vk::BufferUsageFlags,
        memory_usage: vma::MemoryUsage,
        allocation_flags: vma::AllocationCreateFlags,
    ) -> Self {
        let vk_buffer_info = vk::BufferCreateInfo {
            s_type: vk::StructureType::BUFFER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::BufferCreateFlags::empty(),
            size: size.try_into().unwrap(),
            usage: buffer_usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
        };

        let vma_alloc_info = vma::AllocationCreateInfo {
            flags: allocation_flags,
            usage: memory_usage,
            required_flags: vk::MemoryPropertyFlags::empty(),
            preferred_flags: vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            user_data: 0,
            priority: 0.0,
        };

        let (vk_buffer, vma_allocation) = unsafe {
            allocator
                .create_buffer(&vk_buffer_info, &vma_alloc_info)
                .expect("failed to create and allocate buffer")
        };

        let vma_allocation_info = allocator.get_allocation_info(&vma_allocation);

        Self {
            device,
            allocator,
            vk_buffer,
            vma_allocation,
            vma_allocation_info,
        }
    }

    pub fn get_device_address<'t>(&'t self) -> DeviceAddress<'t> {
        let vk_addr_info = vk::BufferDeviceAddressInfo {
            s_type: vk::StructureType::BUFFER_DEVICE_ADDRESS_INFO,
            p_next: std::ptr::null(),
            buffer: self.vk_buffer,
        };

        let vk_device_address = unsafe {
            self.device
                .get_ash_handle()
                .get_buffer_device_address(&vk_addr_info)
        };

        DeviceAddress::new(&self, vk_device_address)
    }

    pub fn copy_nonoverlapping<T>(&self, src: &[T]) -> () {
        unsafe {
            let size = size_of::<T>() * src.len();
            let dst = self.vma_allocation_info.mapped_data;

            std::ptr::copy_nonoverlapping(src.as_ptr() as *const c_void, dst, size);
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
            self.allocator
                .destroy_buffer(self.vk_buffer, &mut self.vma_allocation);
        }
    }
}

pub struct DeviceAddress<'t> {
    buffer: &'t Buffer,
    vk_device_address: vk::DeviceAddress,
}

impl DeviceAddress<'_> {
    pub fn new<'t>(buffer: &'t Buffer, vk_device_address: vk::DeviceAddress) -> DeviceAddress<'t> {
        DeviceAddress::<'t> {
            buffer,
            vk_device_address,
        }
    }
}

impl HasRawVkHandle<vk::DeviceAddress> for DeviceAddress<'_> {
    unsafe fn get_vk_handle(&self) -> vk::DeviceAddress {
        self.vk_device_address
    }
}
