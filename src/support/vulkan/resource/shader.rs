use crate::vulkan::VulkanContext;
use ash::{version::DeviceV1_0, vk};
use snafu::{ResultExt, Snafu};
use std::{
    ffi::{CStr, CString},
    sync::Arc,
};

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Failed to find shader file path '{}': {}", path, source))]
    FindShaderFilePath {
        path: String,
        source: std::io::Error,
    },

    #[snafu(display("Failed to read SPIR-V shader source from bytes: {}", source))]
    ReadShaderSourceBytes { source: std::io::Error },

    #[snafu(display("Failed to create shader module: {}", source))]
    CreateShaderModule { source: ash::vk::Result },
}

// TODO: Make a shader cache with Arc's to each shader, addressable via a string name
//       Then shader sets can just be a collection of cloned Arc's
pub struct ShaderSet {
    pub vertex_shader: Option<Shader>,
    pub fragment_shader: Option<Shader>,
    pub geometry_shader: Option<Shader>,
    pub tessellation_evaluation_shader: Option<Shader>,
    pub tessellation_control_shader: Option<Shader>,
    entry_point_name: CString,
    context: Arc<VulkanContext>,
}

impl ShaderSet {
    pub const SHADER_ENTRY_POINT_NAME: &'static str = "main";

    pub fn new(context: Arc<VulkanContext>) -> Result<Self> {
        let entry_point_name = CString::new(ShaderSet::SHADER_ENTRY_POINT_NAME.to_string())
            .expect("Failed to create CString for shader entry point name!");

        let shader_set = Self {
            context,
            entry_point_name,
            vertex_shader: None,
            fragment_shader: None,
            geometry_shader: None,
            tessellation_evaluation_shader: None,
            tessellation_control_shader: None,
        };

        Ok(shader_set)
    }

    pub fn vertex_shader(mut self, path: &str) -> Result<Self> {
        let vertex_shader = self.create_shader(&path, vk::ShaderStageFlags::VERTEX)?;
        self.vertex_shader = Some(vertex_shader);
        Ok(self)
    }

    pub fn fragment_shader(mut self, path: &str) -> Result<Self> {
        let fragment_shader = self.create_shader(&path, vk::ShaderStageFlags::FRAGMENT)?;
        self.fragment_shader = Some(fragment_shader);
        Ok(self)
    }

    pub fn geometry_shader(mut self, path: &str) -> Result<Self> {
        let geometry_shader = self.create_shader(&path, vk::ShaderStageFlags::GEOMETRY)?;
        self.geometry_shader = Some(geometry_shader);
        Ok(self)
    }

    pub fn tessellation_control_shader(mut self, path: &str) -> Result<Self> {
        let tessellation_control_shader =
            self.create_shader(&path, vk::ShaderStageFlags::TESSELLATION_CONTROL)?;
        self.tessellation_control_shader = Some(tessellation_control_shader);
        Ok(self)
    }

    pub fn tessellation_evaluation_shader(mut self, path: &str) -> Result<Self> {
        let tessellation_evaluation_shader =
            self.create_shader(&path, vk::ShaderStageFlags::TESSELLATION_EVALUATION)?;
        self.tessellation_evaluation_shader = Some(tessellation_evaluation_shader);
        Ok(self)
    }

    fn create_shader(&self, path: &str, stage_flags: vk::ShaderStageFlags) -> Result<Shader> {
        let shader = Shader::from_file(
            self.context.clone(),
            path,
            stage_flags,
            &self.entry_point_name,
        )?;

        Ok(shader)
    }
}

pub struct Shader {
    context: Arc<VulkanContext>,
    module: vk::ShaderModule,
    state_info: vk::PipelineShaderStageCreateInfo,
}

impl Shader {
    // TODO: Refactor this to have less parameters
    pub fn from_file(
        context: Arc<VulkanContext>,
        path: &str,
        flags: vk::ShaderStageFlags,
        entry_point_name: &CStr,
    ) -> Result<Self> {
        let mut shader_file = std::fs::File::open(path).context(FindShaderFilePath { path })?;
        let shader_source = ash::util::read_spv(&mut shader_file).context(ReadShaderSourceBytes)?;
        let shader_create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&shader_source)
            .build();
        let module = unsafe {
            context
                .logical_device()
                .logical_device()
                .create_shader_module(&shader_create_info, None)
                .context(CreateShaderModule)?
        };

        let state_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(flags)
            .module(module)
            .name(entry_point_name)
            .build();

        let shader = Shader {
            module,
            context,
            state_info,
        };

        Ok(shader)
    }

    pub fn state_info(&self) -> vk::PipelineShaderStageCreateInfo {
        self.state_info
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.context
                .logical_device()
                .logical_device()
                .destroy_shader_module(self.module, None);
        }
    }
}
