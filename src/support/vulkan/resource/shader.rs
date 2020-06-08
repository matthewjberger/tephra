use crate::vulkan::VulkanContext;
use ash::{version::DeviceV1_0, vk};
use derive_builder::Builder;
use snafu::{ResultExt, Snafu};
use std::{
    collections::HashMap,
    ffi::CString,
    ops::{Deref, DerefMut},
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

pub type ShaderMap = HashMap<String, Arc<Shader>>;

#[derive(Default)]
pub struct ShaderCache(ShaderMap);

impl Deref for ShaderCache {
    type Target = ShaderMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ShaderCache {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ShaderCache {
    pub fn add_shader(
        &mut self,
        context: Arc<VulkanContext>,
        path: &str,
        stage_flags: vk::ShaderStageFlags,
    ) -> Result<()> {
        let shader =
            Shader::from_file(context, &path, stage_flags, Shader::SHADER_ENTRY_POINT_NAME)?;

        self.insert(path.to_string(), Arc::new(shader));

        Ok(())
    }
}
#[derive(Builder, Clone)]
#[builder(setter(into, strip_option))]
pub struct ShaderSet {
    pub vertex_shader: Arc<Shader>,

    #[builder(default)]
    pub fragment_shader: Option<Arc<Shader>>,

    #[builder(default)]
    pub geometry_shader: Option<Arc<Shader>>,

    #[builder(default)]
    pub tessellation_evaluation_shader: Option<Arc<Shader>>,

    #[builder(default)]
    pub tessellation_control_shader: Option<Arc<Shader>>,
}

pub struct Shader {
    context: Arc<VulkanContext>,
    module: vk::ShaderModule,
    state_info: vk::PipelineShaderStageCreateInfo,
    _entry_point_name: CString,
}

impl Shader {
    pub const SHADER_ENTRY_POINT_NAME: &'static str = "main";

    pub fn from_file(
        context: Arc<VulkanContext>,
        path: &str,
        flags: vk::ShaderStageFlags,
        entry_point_name: &str,
    ) -> Result<Self> {
        let entry_point_name = CString::new(entry_point_name)
            .expect("Failed to create CString for shader entry point name!");
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
            .name(&entry_point_name)
            .build();

        let shader = Shader {
            module,
            context,
            state_info,
            _entry_point_name: entry_point_name,
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
