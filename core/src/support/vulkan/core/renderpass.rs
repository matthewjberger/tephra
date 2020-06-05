use crate::vulkan::VulkanContext;
use ash::{version::DeviceV1_0, vk};
use snafu::{ResultExt, Snafu};
use std::sync::Arc;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Failed to create render pass: {}", source))]
    CreateRenderPass { source: ash::vk::Result },
}

pub struct RenderPass {
    render_pass: vk::RenderPass,
    context: Arc<VulkanContext>,
}

impl RenderPass {
    pub fn new(
        context: Arc<VulkanContext>,
        create_info: &vk::RenderPassCreateInfo,
    ) -> Result<Self> {
        let render_pass = unsafe {
            context
                .logical_device()
                .logical_device()
                .create_render_pass(&create_info, None)
        }
        .context(CreateRenderPass {})?;

        let render_pass = Self {
            render_pass,
            context,
        };

        Ok(render_pass)
    }

    pub fn render_pass(&self) -> vk::RenderPass {
        self.render_pass
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.context
                .logical_device()
                .logical_device()
                .destroy_render_pass(self.render_pass, None);
        }
    }
}
