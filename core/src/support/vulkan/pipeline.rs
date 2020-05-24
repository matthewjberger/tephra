use crate::vulkan::{
    DescriptorSetLayout, GraphicsPipeline, PipelineLayout, Shader, VulkanContext, VulkanSwapchain,
};
use ash::{version::DeviceV1_0, vk};
use std::{ffi::CString, sync::Arc};

pub struct PipelineRenderer {
    pub command_buffer: vk::CommandBuffer,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set: vk::DescriptorSet,
    pub vertex_buffer: vk::Buffer,
    pub index_buffer: Option<vk::Buffer>,
    pub dynamic_alignment: Option<u64>,
}

impl PipelineRenderer {
    pub fn bind_geometry_buffers(&self, device: &ash::Device) {
        let offsets = [0];
        let vertex_buffers = [self.vertex_buffer];

        unsafe {
            device.cmd_bind_vertex_buffers(self.command_buffer, 0, &vertex_buffers, &offsets);

            if let Some(index_buffer) = self.index_buffer {
                device.cmd_bind_index_buffer(
                    self.command_buffer,
                    index_buffer,
                    0,
                    vk::IndexType::UINT32,
                );
            }
        }
    }

    pub fn bind_descriptor_set(&self, device: &ash::Device) {
        unsafe {
            device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );
        }
    }
}

// TODO: Move shader paths into separate struct to be constructed with the builder pattern
pub struct RenderPipelineSettings {
    pub vertex_state_info: vk::PipelineVertexInputStateCreateInfo,
    pub descriptor_set_layout: DescriptorSetLayout,
    pub vertex_shader_path: String,
    pub fragment_shader_path: String,
}

pub struct RenderPipeline {
    pub pipeline: GraphicsPipeline,
}

impl RenderPipeline {
    pub fn new(
        context: Arc<VulkanContext>,
        swapchain: &VulkanSwapchain,
        settings: &RenderPipelineSettings,
    ) -> Self {
        let (vertex_shader, fragment_shader, _shader_entry_point_name) =
            Self::create_shaders(context.clone(), settings);
        let shader_state_info = [vertex_shader.state_info(), fragment_shader.state_info()];

        let input_assembly_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        let rasterizer_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build();

        let multisampling_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(true)
            .rasterization_samples(context.max_usable_samples())
            .min_sample_shading(0.2)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(false)
            .front(Default::default())
            .back(Default::default())
            .build();

        let color_blend_attachments = Self::create_color_blend_attachments();

        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .build();

        let descriptor_set_layout = &settings.descriptor_set_layout;
        let pipeline_layout = Self::create_pipeline_layout(context.clone(), descriptor_set_layout);

        let mut viewport_create_info = vk::PipelineViewportStateCreateInfo::default();
        viewport_create_info.viewport_count = 1;
        viewport_create_info.scissor_count = 1;

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::builder()
            .flags(vk::PipelineDynamicStateCreateFlags::empty())
            .dynamic_states(&dynamic_states)
            .build();

        let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_state_info)
            .vertex_input_state(&settings.vertex_state_info)
            .input_assembly_state(&input_assembly_create_info)
            .rasterization_state(&rasterizer_create_info)
            .multisample_state(&multisampling_create_info)
            .depth_stencil_state(&depth_stencil_info)
            .color_blend_state(&color_blending_info)
            .viewport_state(&viewport_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .layout(pipeline_layout.layout())
            .render_pass(swapchain.render_pass.render_pass())
            .subpass(0)
            .build();

        let pipeline = GraphicsPipeline::new(context, pipeline_create_info, pipeline_layout);

        Self { pipeline }
    }

    fn create_shaders(
        context: Arc<VulkanContext>,
        settings: &RenderPipelineSettings,
    ) -> (Shader, Shader, CString) {
        let shader_entry_point_name =
            CString::new("main").expect("Failed to create CString for shader entry point name!");

        let vertex_shader = Shader::from_file(
            context.clone(),
            &settings.vertex_shader_path,
            vk::ShaderStageFlags::VERTEX,
            &shader_entry_point_name,
        )
        .expect("Failed to create vertex shader!");

        let fragment_shader = Shader::from_file(
            context,
            &settings.fragment_shader_path,
            vk::ShaderStageFlags::FRAGMENT,
            &shader_entry_point_name,
        )
        .expect("Failed to create fragment shader!");

        (vertex_shader, fragment_shader, shader_entry_point_name)
    }

    pub fn create_color_blend_attachments() -> [vk::PipelineColorBlendAttachmentState; 1] {
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false)
            .src_color_blend_factor(vk::BlendFactor::ONE)
            .dst_color_blend_factor(vk::BlendFactor::ZERO)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build();
        [color_blend_attachment]
    }

    pub fn create_pipeline_layout(
        context: Arc<VulkanContext>,
        descriptor_set_layout: &DescriptorSetLayout,
    ) -> PipelineLayout {
        let descriptor_set_layouts = [descriptor_set_layout.layout()];

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&descriptor_set_layouts)
            .build();

        PipelineLayout::new(context, pipeline_layout_create_info)
    }

    pub fn bind(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline(),
            );
        }
    }
}
