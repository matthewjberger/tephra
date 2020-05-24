use ash::{version::DeviceV1_0, vk};
use nalgebra_glm as glm;
use std::{ffi::CString, mem, sync::Arc};
use support::{
    app::{run_app, App},
    vulkan::{
        Buffer, Command, CommandPool, DescriptorPool, DescriptorSetLayout, GeometryBuffer,
        GraphicsPipeline, PipelineLayout, Renderer, Shader, VulkanContext, VulkanSwapchain,
    },
};

#[derive(Default)]
struct DemoApp {
    context: Option<Arc<VulkanContext>>,
    triangle: Option<Triangle>,
    pipeline: Option<TrianglePipeline>,
    pipeline_data: Option<TrianglePipelineData>,
    rotation: f32,
}

impl Drop for DemoApp {
    fn drop(&mut self) {
        self.context
            .as_ref()
            .expect("Failed to get vulkan context!")
            .logical_device()
            .wait_idle();
    }
}

impl App for DemoApp {
    fn initialize(&mut self, renderer: &mut Renderer) {
        self.context = Some(renderer.context.clone());
        self.triangle = Some(Triangle::new(&renderer.command_pool));
        self.pipeline_data = Some(TrianglePipelineData::new(renderer.context.clone()));
        self.recreate_pipelines(renderer.context.clone(), renderer.vulkan_swapchain());
        renderer.record_all_command_buffers(self as &mut dyn Command);
    }

    fn update(&mut self, renderer: &mut Renderer, _: f64) {
        self.rotation += 0.05;
        if (self.rotation - 360.0) > 0.001 {
            self.rotation = 0.0;
        }
        let model = glm::rotate(
            &glm::Mat4::identity(),
            self.rotation.to_radians(),
            &glm::vec3(0.0, 1.0, 0.0),
        );

        let projection = glm::perspective_zo(
            renderer
                .vulkan_swapchain()
                .swapchain
                .properties()
                .aspect_ratio(),
            90_f32.to_radians(),
            0.1_f32,
            1000_f32,
        );

        let view = glm::look_at(
            &glm::vec3(0.0, 0.0, -4.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::vec3(0.0, 1.0, 0.0),
        );

        let ubo = UniformBufferObject {
            model,
            view,
            projection,
        };
        let ubos = [ubo];

        if let Some(pipeline_data) = self.pipeline_data.as_ref() {
            pipeline_data.uniform_buffer.upload_to_buffer(&ubos, 0);
        }
    }

    fn draw(&mut self, renderer: &mut Renderer, window_dimensions: glm::Vec2) {
        renderer.render(window_dimensions, self as &mut dyn Command);
    }
}

impl Command for DemoApp {
    fn issue_commands(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        let pipeline = self.pipeline.as_ref().expect("Failed to get pipeline!");
        let triangle_renderer = TriangleRenderer::new(
            command_buffer,
            &pipeline,
            self.pipeline_data
                .as_ref()
                .expect("Failed to get pipeline data!"),
        );
        pipeline.bind(device, command_buffer);
        triangle_renderer.draw(
            device,
            command_buffer,
            &self
                .triangle
                .as_ref()
                .expect("Failed to get triangle data!")
                .buffers,
        );
    }

    fn recreate_pipelines(&mut self, context: Arc<VulkanContext>, swapchain: &VulkanSwapchain) {
        self.pipeline = None;
        self.pipeline = Some(TrianglePipeline::new(context, &swapchain));
    }
}

fn main() {
    run_app(DemoApp::default(), "Triangle");
}

pub struct Triangle {
    buffers: GeometryBuffer,
}

impl Triangle {
    pub fn new(command_pool: &CommandPool) -> Self {
        let vertices = vec![1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 0.0, -1.0, 0.0];
        let indices = vec![0, 1, 2];
        let buffers = GeometryBuffer::new(command_pool, &vertices, Some(&indices));
        Self { buffers }
    }

    pub fn create_vertex_attributes() -> [vk::VertexInputAttributeDescription; 1] {
        let position_description = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        [position_description]
    }

    pub fn create_vertex_input_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        let vertex_input_binding_description = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride((3 * std::mem::size_of::<f32>()) as _)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();
        [vertex_input_binding_description]
    }
}

pub struct TrianglePipeline {
    pub pipeline: GraphicsPipeline,
}

impl TrianglePipeline {
    pub fn new(context: Arc<VulkanContext>, swapchain: &VulkanSwapchain) -> Self {
        let (vertex_shader, fragment_shader, _shader_entry_point_name) =
            Self::create_shaders(context.clone());
        let shader_state_info = [vertex_shader.state_info(), fragment_shader.state_info()];

        let descriptions = Triangle::create_vertex_input_descriptions();
        let attributes = Triangle::create_vertex_attributes();
        let vertex_input_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&descriptions)
            .vertex_attribute_descriptions(&attributes)
            .build();

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

        let descriptor_set_layout = TrianglePipelineData::descriptor_set_layout(context.clone());
        let pipeline_layout = Self::create_pipeline_layout(context.clone(), &descriptor_set_layout);

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
            .vertex_input_state(&vertex_input_create_info)
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

        let pipeline = GraphicsPipeline::new(
            context,
            pipeline_create_info,
            pipeline_layout,
            descriptor_set_layout,
        );

        Self { pipeline }
    }

    fn create_shaders(context: Arc<VulkanContext>) -> (Shader, Shader, CString) {
        let shader_entry_point_name =
            CString::new("main").expect("Failed to create CString for shader entry point name!");

        let vertex_shader = Shader::from_file(
            context.clone(),
            "core/assets/shaders/triangle/triangle.vert.spv",
            vk::ShaderStageFlags::VERTEX,
            &shader_entry_point_name,
        )
        .expect("Failed to create vertex shader!");

        let fragment_shader = Shader::from_file(
            context,
            "core/assets/shaders/triangle/triangle.frag.spv",
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

#[derive(Clone, Copy)]
pub struct UniformBufferObject {
    pub model: glm::Mat4,
    pub view: glm::Mat4,
    pub projection: glm::Mat4,
}

pub struct TrianglePipelineData {
    pub descriptor_pool: DescriptorPool,
    pub uniform_buffer: Buffer,
    pub descriptor_set: vk::DescriptorSet,
}

impl TrianglePipelineData {
    pub fn new(context: Arc<VulkanContext>) -> Self {
        let descriptor_set_layout = Self::descriptor_set_layout(context.clone());
        let descriptor_pool = Self::create_descriptor_pool(context.clone());
        let descriptor_set =
            descriptor_pool.allocate_descriptor_sets(descriptor_set_layout.layout(), 1)[0];

        let uniform_buffer = Buffer::new_mapped_basic(
            context.clone(),
            mem::size_of::<UniformBufferObject>() as _,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu,
        );

        let data = TrianglePipelineData {
            descriptor_pool,
            uniform_buffer,
            descriptor_set,
        };

        data.update_descriptor_set(context);

        data
    }

    pub fn descriptor_set_layout(context: Arc<VulkanContext>) -> DescriptorSetLayout {
        let ubo_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)
            .build();

        let bindings = [ubo_binding];

        let layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&bindings)
            .build();

        DescriptorSetLayout::new(context, layout_create_info)
    }

    fn create_descriptor_pool(context: Arc<VulkanContext>) -> DescriptorPool {
        let ubo_pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
        };

        let pool_sizes = [ubo_pool_size];

        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1)
            .build();

        DescriptorPool::new(context, pool_info)
    }

    fn update_descriptor_set(&self, context: Arc<VulkanContext>) {
        let uniform_buffer_size = mem::size_of::<UniformBufferObject>() as vk::DeviceSize;
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(self.uniform_buffer.buffer())
            .offset(0)
            .range(uniform_buffer_size)
            .build();
        let buffer_infos = [buffer_info];

        let ubo_descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(self.descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_infos)
            .build();

        let descriptor_writes = vec![ubo_descriptor_write];

        unsafe {
            context
                .logical_device()
                .logical_device()
                .update_descriptor_sets(&descriptor_writes, &[])
        }
    }
}

pub struct TriangleRenderer {
    command_buffer: vk::CommandBuffer,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: vk::DescriptorSet,
}

impl TriangleRenderer {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        pipeline: &TrianglePipeline,
        pipeline_data: &TrianglePipelineData,
    ) -> Self {
        Self {
            command_buffer,
            pipeline_layout: pipeline.pipeline.layout(),
            descriptor_set: pipeline_data.descriptor_set,
        }
    }

    pub fn draw(
        &self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        buffers: &GeometryBuffer,
    ) {
        let offsets = [0];
        let vertex_buffers = [buffers.vertex_buffer.buffer()];

        unsafe {
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            device.cmd_bind_index_buffer(
                command_buffer,
                buffers
                    .index_buffer
                    .as_ref()
                    .expect("Failed to get an index buffer!")
                    .buffer(),
                0,
                vk::IndexType::UINT32,
            );

            device.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.descriptor_set],
                &[],
            );

            device.cmd_draw_indexed(self.command_buffer, 3, 1, 0, 0, 1);
        }
    }
}
