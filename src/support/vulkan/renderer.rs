use crate::vulkan::{VulkanContext, VulkanSwapchain, SynchronizationSet, CommandPool};
use std::sync::Arc;

pub struct Renderer {
    pub context: Arc<VulkanContext>,
    vulkan_swapchain: Option<VulkanSwapchain>,
    pub synchronization_set: SynchronizationSet,
    pub current_frame: usize,
    pub command_pool: CommandPool,
    pub transient_command_pool: CommandPool,
}

impl Renderer {
    pub fn vulkan_swapchain(&self) -> &VulkanSwapchain {
        self.vulkan_swapchain
            .as_ref()
            .expect("Failed to get vulkan swapchain!")
    }
}
