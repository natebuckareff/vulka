use super::{Device, HasRawVkHandle, ImageView};
use ash::vk;
use std::sync::Arc;
use vma::Alloc;

pub struct Image {
    device: Arc<Device>,
    vk_image: vk::Image,
    image_type: vk::ImageType,
    format: vk::Format,
    extent: vk::Extent3D,
    allocated: Option<AllocatedImage>,
}

pub struct AllocatedImage {
    allocator: Arc<vma::Allocator>,
    vma_allocation: vma::Allocation,
    vma_allocation_info: vma::AllocationInfo,
}

impl Image {
    pub fn new(
        device: Arc<Device>,
        allocator: Arc<vma::Allocator>,
        image_type: vk::ImageType,
        format: vk::Format,
        extent: vk::Extent3D,
        mip_levels: u32,
        array_layers: u32,
        samples: vk::SampleCountFlags,
        tiling: vk::ImageTiling,
        image_usage: vk::ImageUsageFlags,
        memory_usage: vma::MemoryUsage,
        allocation_flags: vma::AllocationCreateFlags,
        required_flags: vk::MemoryPropertyFlags,
    ) -> Arc<Self> {
        let vk_image_info = vk::ImageCreateInfo {
            s_type: vk::StructureType::IMAGE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::ImageCreateFlags::empty(),
            image_type,
            format,
            extent,
            mip_levels,
            array_layers,
            samples,
            tiling,
            usage: image_usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_index_count: 0,
            p_queue_family_indices: std::ptr::null(),
            initial_layout: vk::ImageLayout::UNDEFINED,
        };

        let vma_create_info = vma::AllocationCreateInfo {
            flags: allocation_flags,
            usage: memory_usage,
            required_flags,
            preferred_flags: vk::MemoryPropertyFlags::empty(),
            memory_type_bits: 0,
            user_data: 0,
            priority: 0.0,
        };

        let (vk_image, vma_allocation) = unsafe {
            allocator
                .create_image(&vk_image_info, &vma_create_info)
                .expect("failed to create and allocate image")
        };

        let vma_allocation_info = allocator.get_allocation_info(&vma_allocation);

        Arc::new(Self {
            device,
            vk_image,
            image_type,
            format,
            extent,
            allocated: Some(AllocatedImage {
                allocator,
                vma_allocation,
                vma_allocation_info,
            }),
        })
    }

    pub fn image_type(&self) -> &vk::ImageType {
        &self.image_type
    }

    pub fn format(&self) -> &vk::Format {
        &self.format
    }

    pub fn extent(&self) -> &vk::Extent3D {
        &self.extent
    }

    // Create an image that is owned by a swapchain
    pub fn from_swapchain(
        device: Arc<Device>,
        vk_image: vk::Image,
        image_type: vk::ImageType,
        format: vk::Format,
        extent: vk::Extent3D,
    ) -> Arc<Self> {
        Arc::new(Self {
            device: device.clone(),
            vk_image,
            image_type,
            format,
            extent,
            allocated: None,
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn get_default_view(self: &Arc<Self>, aspect_mask: vk::ImageAspectFlags) -> Arc<ImageView> {
        let view_type = match self.image_type {
            vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
            vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
            vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
            _ => unreachable!(),
        };

        ImageView::new(
            self.clone(),
            view_type,
            self.format,
            vk::ImageSubresourceRange {
                aspect_mask,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        )
    }
}

impl HasRawVkHandle<vk::Image> for Image {
    unsafe fn get_vk_handle(&self) -> vk::Image {
        self.vk_image
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        if let Some(allocated) = &mut self.allocated {
            let AllocatedImage {
                allocator,
                vma_allocation,
                ..
            } = allocated;
            unsafe { allocator.destroy_image(self.vk_image, vma_allocation) };
        }
    }
}
