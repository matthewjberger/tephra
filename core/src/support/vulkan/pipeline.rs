use crate::vulkan::{
    DescriptorSetLayout, GraphicsPipeline, PipelineLayout, ShaderSet, VulkanContext,
};
use ash::{version::DeviceV1_0, vk};
use std::sync::Arc;

// TODO: Add a builder for this struct
// TODO: Move shader paths into separate struct to be constructed with the builder pattern
#[derive(Clone)]
pub struct RenderPipelineSettings {
    pub render_pass: vk::RenderPass,
    pub vertex_state_info: vk::PipelineVertexInputStateCreateInfo,
    pub descriptor_set_layout: Arc<DescriptorSetLayout>,
    pub blended: bool,
    pub depth_test_enabled: bool,
    pub stencil_test_enabled: bool,
    pub stencil_front_state: vk::StencilOpState,
    pub stencil_back_state: vk::StencilOpState,
    pub push_constant_range: Option<vk::PushConstantRange>,
    pub shader_set: Arc<ShaderSet>,
}

impl RenderPipelineSettings {
    pub fn new(
        render_pass: vk::RenderPass,
        vertex_state_info: vk::PipelineVertexInputStateCreateInfo,
        descriptor_set_layout: Arc<DescriptorSetLayout>,
        shader_set: Arc<ShaderSet>,
    ) -> Self {
        Self {
            render_pass,
            vertex_state_info,
            descriptor_set_layout,
            shader_set,
            blended: false,
            depth_test_enabled: true,
            stencil_test_enabled: false,
            stencil_front_state: vk::StencilOpState::default(),
            stencil_back_state: vk::StencilOpState::default(),
            push_constant_range: None,
        }
    }

    pub fn blended(mut self, blended: bool) -> Self {
        self.blended = blended;
        self
    }

    pub fn depth_test_enabled(mut self, depth_test_enabled: bool) -> Self {
        self.depth_test_enabled = depth_test_enabled;
        self
    }

    pub fn stencil_test_enabled(mut self, stencil_test_enabled: bool) -> Self {
        self.stencil_test_enabled = stencil_test_enabled;
        self
    }

    pub fn stencil_front_state(mut self, stencil_front_state: vk::StencilOpState) -> Self {
        self.stencil_front_state = stencil_front_state;
        self
    }

    pub fn stencil_back_state(mut self, stencil_back_state: vk::StencilOpState) -> Self {
        self.stencil_back_state = stencil_back_state;
        self
    }

    pub fn push_constant_range(mut self, push_constant_range: vk::PushConstantRange) -> Self {
        self.push_constant_range = Some(push_constant_range);
        self
    }
}

pub struct RenderPipeline {
    pub settings: RenderPipelineSettings,
    pub pipeline: GraphicsPipeline,
}

impl RenderPipeline {
    pub fn new(context: Arc<VulkanContext>, settings: RenderPipelineSettings) -> Self {
        let shader_state_info = [
            settings
                .shader_set
                .vertex_shader
                .as_ref()
                .expect("Failed to lookup vertex shader!")
                .state_info(),
            settings
                .shader_set
                .fragment_shader
                .as_ref()
                .expect("Failed to lookup fragment shader!")
                .state_info(),
        ];

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
            .depth_test_enable(settings.depth_test_enabled)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .depth_bounds_test_enable(false)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0)
            .stencil_test_enable(settings.stencil_test_enabled)
            .front(settings.stencil_front_state)
            .back(settings.stencil_back_state)
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
            .render_pass(settings.render_pass)
            .subpass(0)
            .build();

        let pipeline = GraphicsPipeline::new(context, pipeline_create_info, pipeline_layout);

        Self { pipeline, settings }
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
