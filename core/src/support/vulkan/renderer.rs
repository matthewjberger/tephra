use crate::vulkan::{
    core::sync::synchronization_set::SynchronizationSetConstants, CommandPool, SynchronizationSet,
    VulkanContext, VulkanSwapchain,
};
use ash::{version::DeviceV1_0, vk};
use nalgebra_glm as glm;
use std::sync::Arc;
use winit::window::Window;

pub trait Command {
    fn issue_commands(&mut self, _: &ash::Device, _: vk::CommandBuffer) {}
    fn recreate_pipelines(&mut self, _: Arc<VulkanContext>, _: &VulkanSwapchain) {}
}

pub struct Renderer {
    pub context: Arc<VulkanContext>,
    pub vulkan_swapchain: Option<VulkanSwapchain>,
    pub synchronization_set: SynchronizationSet,
    pub current_frame: usize,
    pub command_pool: CommandPool,
    pub transient_command_pool: CommandPool,
}

impl Renderer {
    pub fn new(context: Arc<VulkanContext>, window: &Window) -> Self {
        let synchronization_set =
            SynchronizationSet::new(context.clone()).expect("Failed to create sync objects");

        let command_pool = CommandPool::new(
            context.clone(),
            vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
        );

        let transient_command_pool =
            CommandPool::new(context.clone(), vk::CommandPoolCreateFlags::TRANSIENT);

        let logical_size = window.inner_size();
        let dimensions = [logical_size.width as u32, logical_size.height as u32];

        let vulkan_swapchain = Some(VulkanSwapchain::new(
            context.clone(),
            dimensions,
            &command_pool,
        ));

        Self {
            context,
            vulkan_swapchain,
            synchronization_set,
            current_frame: 0,
            command_pool,
            transient_command_pool,
        }
    }

    pub fn vulkan_swapchain(&self) -> &VulkanSwapchain {
        self.vulkan_swapchain
            .as_ref()
            .expect("Failed to get vulkan swapchain!")
    }

    pub fn allocate_command_buffers(&mut self) {
        // Allocate one command buffer per swapchain image
        let number_of_framebuffers = self.vulkan_swapchain().framebuffers.len();
        self.command_pool
            .allocate_command_buffers(number_of_framebuffers as _);
    }

    pub fn record_all_command_buffers(&self, command: &mut dyn Command) {
        // Create a single render pass per swapchain image that will draw each mesh
        self.command_pool
            .command_buffers()
            .iter()
            .enumerate()
            .for_each(|(index, buffer)| {
                let command_buffer = *buffer;
                let framebuffer = self.vulkan_swapchain().framebuffers[index].framebuffer();
                self.record_single_command_buffer(framebuffer, command_buffer, command);
            });
    }

    pub fn render(&mut self, window_dimensions: glm::Vec2, command: &mut dyn Command) {
        let context = self.context.clone();

        let current_frame_synchronization = self
            .synchronization_set
            .current_frame_synchronization(self.current_frame);

        context
            .logical_device()
            .wait_for_fence(&current_frame_synchronization);

        // Acquire the next image from the swapchain
        let image_index_result = self.vulkan_swapchain().swapchain.acquire_next_image(
            current_frame_synchronization.image_available(),
            vk::Fence::null(),
        );

        let image_index = match image_index_result {
            Ok((image_index, _)) => image_index,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(window_dimensions, command);
                return;
            }
            Err(error) => panic!("Error while acquiring next image. Cause: {}", error),
        };
        let image_indices = [image_index];

        context
            .logical_device()
            .reset_fence(&current_frame_synchronization);

        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        self.command_pool.submit_command_buffer(
            image_index as usize,
            self.context.graphics_queue(),
            &wait_stages,
            &current_frame_synchronization,
        );

        let swapchain_presentation_result =
            self.vulkan_swapchain().swapchain.present_rendered_image(
                &current_frame_synchronization,
                &image_indices,
                self.context.present_queue(),
            );

        match swapchain_presentation_result {
            Ok(is_suboptimal) if is_suboptimal => {
                self.recreate_swapchain(window_dimensions, command);
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain(window_dimensions, command);
            }
            Err(error) => panic!("Failed to present queue. Cause: {}", error),
            _ => {}
        }

        self.current_frame +=
            (1 + self.current_frame) % SynchronizationSet::MAX_FRAMES_IN_FLIGHT as usize;
    }

    pub fn recreate_swapchain(&mut self, window_dimensions: glm::Vec2, command: &mut dyn Command) {
        self.context.logical_device().wait_idle();

        self.vulkan_swapchain = None;
        let new_swapchain = VulkanSwapchain::new(
            self.context.clone(),
            [window_dimensions.x as _, window_dimensions.y as _],
            &self.command_pool,
        );
        self.vulkan_swapchain = Some(new_swapchain);

        command.recreate_pipelines(self.context.clone(), self.vulkan_swapchain());
        self.record_all_command_buffers(command);
    }

    pub fn update_viewport(&self, command_buffer: vk::CommandBuffer) {
        let device = self.context.logical_device().logical_device();
        let extent = self.vulkan_swapchain().swapchain.properties().extent;

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as _,
            height: extent.height as _,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let viewports = [viewport];

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };
        let scissors = [scissor];

        unsafe {
            device.cmd_set_viewport(command_buffer, 0, &viewports);
            device.cmd_set_scissor(command_buffer, 0, &scissors);
        }
    }

    fn record_single_command_buffer(
        &self,
        framebuffer: vk::Framebuffer,
        command_buffer: vk::CommandBuffer,
        command: &mut dyn Command,
    ) {
        let device = self.context.logical_device().logical_device();

        let command_buffer_begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::SIMULTANEOUS_USE)
            .build();
        unsafe {
            device
                .begin_command_buffer(command_buffer, &command_buffer_begin_info)
                .expect("Failed to begin command buffer for the render pass!")
        };

        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.39, 0.58, 0.93, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];

        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.vulkan_swapchain().render_pass.render_pass())
            .framebuffer(framebuffer)
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.vulkan_swapchain().swapchain.properties().extent,
            })
            .clear_values(&clear_values)
            .build();

        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
        }

        self.update_viewport(command_buffer);

        command.issue_commands(device, command_buffer);

        unsafe {
            self.context
                .logical_device()
                .logical_device()
                .cmd_end_render_pass(command_buffer);

            self.context
                .logical_device()
                .logical_device()
                .end_command_buffer(command_buffer)
                .expect("Failed to end the command buffer for a render pass!");
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.context.logical_device().wait_idle();
    }
}
