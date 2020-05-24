use ash::{version::DeviceV1_0, vk};
use nalgebra_glm as glm;
use std::{mem, sync::Arc};
use support::{
    app::{run_app, setup_app, App},
    vulkan::{
        Buffer, Command, CommandPool, DescriptorPool, DescriptorSetLayout, GeometryBuffer,
        PipelineRenderer, RenderPipeline, RenderPipelineSettings, Renderer, VulkanContext,
        VulkanSwapchain,
    },
};

fn main() {
    let (window, event_loop, renderer) = setup_app("Triangle");
    run_app(
        DemoApp::new(renderer.context.clone(), &renderer.command_pool),
        window,
        event_loop,
        renderer,
    );
}

struct DemoApp {
    context: Arc<VulkanContext>,
    triangle: Triangle,
    pipeline: Option<RenderPipeline>,
    pipeline_data: TrianglePipelineData,
    rotation: f32,
}

impl DemoApp {
    pub fn new(context: Arc<VulkanContext>, command_pool: &CommandPool) -> Self {
        Self {
            context: context.clone(),
            triangle: Triangle::new(&command_pool),
            pipeline: None,
            pipeline_data: TrianglePipelineData::new(context),
            rotation: 0.0,
        }
    }
}

impl Drop for DemoApp {
    fn drop(&mut self) {
        self.context.logical_device().wait_idle();
    }
}

impl App for DemoApp {
    fn initialize(&mut self, renderer: &mut Renderer) {
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
            &glm::vec3(0.0, -4.0, 4.0),
            &glm::vec3(0.0, 0.0, 0.0),
            &glm::vec3(0.0, 1.0, 0.0),
        );

        let ubo = UniformBufferObject {
            model,
            view,
            projection,
        };
        let ubos = [ubo];

        self.pipeline_data.uniform_buffer.upload_to_buffer(&ubos, 0);
    }

    fn draw(&mut self, renderer: &mut Renderer, window_dimensions: glm::Vec2) {
        renderer.render(window_dimensions, self as &mut dyn Command);
    }
}

impl Command for DemoApp {
    fn issue_commands(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        let pipeline = self.pipeline.as_ref().expect("Failed to get pipeline!");

        let geometry_buffers = &self.triangle.buffers;

        let pipeline_renderer = PipelineRenderer {
            command_buffer,
            pipeline_layout: pipeline.pipeline.layout(),
            descriptor_set: self.pipeline_data.descriptor_set,
            vertex_buffer: geometry_buffers.vertex_buffer.buffer(),
            index_buffer: if let Some(index_buffer) = geometry_buffers.index_buffer.as_ref() {
                Some(index_buffer.buffer())
            } else {
                None
            },
            dynamic_alignment: None,
        };

        pipeline_renderer.bind_geometry_buffers(device);

        pipeline.bind(device, command_buffer);

        pipeline_renderer.bind_descriptor_set(device);

        unsafe {
            device.cmd_draw_indexed(command_buffer, self.triangle.number_of_indices, 1, 0, 0, 1);
        }
    }

    fn recreate_pipelines(&mut self, context: Arc<VulkanContext>, swapchain: &VulkanSwapchain) {
        let descriptions = Triangle::create_vertex_input_descriptions();
        let attributes = Triangle::create_vertex_attributes();
        let vertex_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&descriptions)
            .vertex_attribute_descriptions(&attributes)
            .build();

        let settings = RenderPipelineSettings {
            vertex_state_info,
            descriptor_set_layout: TrianglePipelineData::descriptor_set_layout(context.clone()),
            vertex_shader_path: "core/assets/shaders/model/model.vert.spv".to_string(),
            fragment_shader_path: "core/assets/shaders/model/model.frag.spv".to_string(),
        };

        self.pipeline = None;
        self.pipeline = Some(RenderPipeline::new(context, swapchain, &settings));
    }
}

pub struct Triangle {
    buffers: GeometryBuffer,
    number_of_indices: u32,
}

impl Triangle {
    pub fn new(command_pool: &CommandPool) -> Self {
        let (models, _) =
            tobj::load_obj("core/assets/models/teapot.obj", false).expect("Failed to load file");
        let vertices = &models[0].mesh.positions;
        let indices = &models[0].mesh.indices;
        let number_of_indices = indices.len() as u32;
        let buffers = GeometryBuffer::new(command_pool, vertices, Some(indices));
        Self {
            buffers,
            number_of_indices,
        }
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
