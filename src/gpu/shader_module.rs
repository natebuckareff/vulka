use super::{Device, HasRawAshHandle, HasRawVkHandle};
use ash::vk;
use shaderc::CompileOptions;
use std::{cell::OnceCell, ffi::CString, sync::Arc};

#[derive(Debug, Clone, Copy)]
pub enum ShaderKind {
    Vertex,
    Fragment,
}

pub struct ShaderModule {
    device: Arc<Device>,
    vk_shader_module: vk::ShaderModule,
    kind: ShaderKind,
    entry_point: &'static str,
    entry_point_cstr: OnceCell<CString>,
    pipeline_shader_stage_create_info: OnceCell<vk::PipelineShaderStageCreateInfo>,
}

impl ShaderModule {
    pub fn new(
        device: &Arc<Device>,
        compiler: &shaderc::Compiler,
        source: &str,
        kind: ShaderKind,
        file_name: &str,
        entry_point: &'static str,
        options: Option<&CompileOptions>,
    ) -> Arc<ShaderModule> {
        let shaderc_kind = match kind {
            ShaderKind::Vertex => shaderc::ShaderKind::Vertex,
            ShaderKind::Fragment => shaderc::ShaderKind::Fragment,
        };

        let artifact = compiler
            .compile_into_spirv(source, shaderc_kind, file_name, entry_point, options)
            .expect("failed to compile shader");

        let bytes = artifact.as_binary_u8();

        let create_info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: vk::ShaderModuleCreateFlags::empty(),
            code_size: bytes.len(),
            p_code: bytes.as_ptr() as *const u32,
        };

        let vk_shader_module = unsafe {
            device
                .get_ash_handle()
                .create_shader_module(&create_info, None)
                .expect("failed to create shader module")
        };

        Arc::new(ShaderModule {
            device: device.clone(),
            vk_shader_module,
            kind,
            entry_point,
            entry_point_cstr: OnceCell::new(),
            pipeline_shader_stage_create_info: OnceCell::new(),
        })
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub fn kind(&self) -> ShaderKind {
        self.kind
    }

    pub fn entry_point(&self) -> &str {
        self.entry_point
    }

    pub fn pipeline_shader_stage_create_info(&self) -> &vk::PipelineShaderStageCreateInfo {
        self.pipeline_shader_stage_create_info.get_or_init(|| {
            let entry_point_cstr = self
                .entry_point_cstr
                .get_or_init(|| CString::new(self.entry_point()).unwrap());

            let stage = match self.kind {
                ShaderKind::Vertex => vk::ShaderStageFlags::VERTEX,
                ShaderKind::Fragment => vk::ShaderStageFlags::FRAGMENT,
            };

            vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                stage,
                module: self.vk_shader_module,
                p_name: entry_point_cstr.as_ptr(),
                p_specialization_info: std::ptr::null(),
            }
        })
    }
}

impl HasRawVkHandle<vk::ShaderModule> for ShaderModule {
    unsafe fn get_vk_handle(&self) -> vk::ShaderModule {
        self.vk_shader_module
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device
                .get_ash_handle()
                .destroy_shader_module(self.vk_shader_module, None);
        }
    }
}
