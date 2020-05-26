use crate::vulkan::{
    DescriptorSetLayout, GraphicsPipeline, PipelineLayout, Shader, VulkanContext, VulkanSwapchain,
};
use ash::{version::DeviceV1_0, vk};
use std::{ffi::CString, sync::Arc};

// TODO: Add a builder for this struct
// TODO: Move shader paths into separate struct to be constructed with the builder pattern
#[derive(Clone)]
pub struct RenderPipelineSettings {
    pub vertex_state_info: vk::PipelineVertexInputStateCreateInfo,
    pub descriptor_set_layout: Arc<DescriptorSetLayout>,
    pub vertex_shader_path: String,
    pub fragment_shader_path: String,
    pub blended: bool,
    pub push_constant_range: Option<vk::PushConstantRange>,
}

pub struct RenderPipeline {
    pub settings: RenderPipelineSettings,
    pub pipeline: GraphicsPipeline,
}

impl RenderPipeline {
    pub fn new(
        context: Arc<VulkanContext>,
        swapchain: &VulkanSwapchain,
        settings: RenderPipelineSettings,
    ) -> Self {
        let (vertex_shader, fragment_shader, _shader_entry_point_name) =
            Self::create_shaders(context.clone(), &settings);
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

        let color_blend_attachments = if settings.blended {
            Self::create_color_blend_attachments_blended()
        } else {
            Self::create_color_blend_attachments_opaque()
        };

        let color_blending_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .build();

        let pipeline_layout = Self::create_pipeline_layout(context.clone(), &settings);

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

        Self { pipeline, settings }
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

    pub fn create_color_blend_attachments_opaque() -> [vk::PipelineColorBlendAttachmentState; 1] {
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

    pub fn create_color_blend_attachments_blended() -> [vk::PipelineColorBlendAttachmentState; 1] {
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD)
            .build();
        [color_blend_attachment]
    }

    pub fn create_pipeline_layout(
        context: Arc<VulkanContext>,
        settings: &RenderPipelineSettings,
    ) -> PipelineLayout {
        let descriptor_set_layouts = [settings.descriptor_set_layout.layout()];

        if let Some(push_constant_range) = settings.push_constant_range.as_ref() {
            let push_constant_ranges = [*push_constant_range];
            let pipeline_layout_create_info_builder = vk::PipelineLayoutCreateInfo::builder()
                .push_constant_ranges(&push_constant_ranges)
                .set_layouts(&descriptor_set_layouts);
            PipelineLayout::new(context, pipeline_layout_create_info_builder.build())
        } else {
            let pipeline_layout_create_info_builder =
                vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);
            PipelineLayout::new(context, pipeline_layout_create_info_builder.build())
        }
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
