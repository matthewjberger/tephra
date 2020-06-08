use ash::{version::DeviceV1_0, vk};
use nalgebra_glm as glm;
use snafu::{ResultExt, Snafu};
use std::{boxed::Box, mem, sync::Arc};
use support::{
    app::{run_app, setup_app, App, AppState},
    camera::FreeCamera,
    vulkan::{
        Buffer, Command, CommandPool, DescriptorPool, DescriptorSetLayout, ObjModel,
        RenderPipeline, RenderPipelineSettings, Renderer, ShaderSet, VulkanContext,
        VulkanSwapchain,
    },
};
use winit::window::Window;

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Failed to create render pipeline: {}", source))]
    CreateRenderPipeline {
        source: support::vulkan::shader::Error,
    },

    #[snafu(display("Failed to create shader: {}", source))]
    CreateShader {
        source: support::vulkan::shader::Error,
    },

    #[snafu(display("Failed to create shader set: {}", source))]
    CreateShaderSet {
        source: support::vulkan::shader::Error,
    },
}

fn main() {
    let (window, event_loop, renderer) = setup_app("Model");
    run_app(
        DemoApp::new(renderer.context.clone(), &renderer.transient_command_pool),
        window,
        event_loop,
        renderer,
    );
}

struct DemoApp {
    context: Arc<VulkanContext>,
    model: ObjModel,
    pipeline: Option<RenderPipeline>,
    pipeline_data: ModelPipelineData,
    rotation: f32,
    camera: FreeCamera,
}

impl DemoApp {
    pub fn new(context: Arc<VulkanContext>, command_pool: &CommandPool) -> Self {
        Self {
            context: context.clone(),
            model: ObjModel::new(&command_pool, "assets/models/teapot.obj"),
            pipeline: None,
            pipeline_data: ModelPipelineData::new(context),
            rotation: 0.0,
            camera: FreeCamera::default(),
        }
    }
}

impl Drop for DemoApp {
    fn drop(&mut self) {
        self.context.logical_device().wait_idle();
    }
}

impl App for DemoApp {
    fn initialize(
        &mut self,
        window: &mut Window,
        renderer: &mut Renderer,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.recreate_pipelines(renderer.context.clone(), renderer.vulkan_swapchain())?;
        renderer.record_all_command_buffers(self as &mut dyn Command);

        window.set_cursor_visible(false);
        window
            .set_cursor_grab(true)
            .expect("Failed to grab cursor!");

        self.camera.position_at(&glm::vec3(0.0, -4.0, -4.0));
        self.camera.look_at(&glm::vec3(0.0, 0.0, 0.0));

        window
            .set_cursor_position(app_state.window_center())
            .expect("Failed to set cursor position!");

        Ok(())
    }

    fn update(
        &mut self,
        window: &mut Window,
        renderer: &mut Renderer,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.camera.update(&app_state);

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

        let ubo = UniformBufferObject {
            model,
            view: self.camera.view_matrix(),
            projection,
        };
        let ubos = [ubo];

        self.pipeline_data
            .uniform_buffer
            .upload_to_buffer(&ubos, 0)
            .unwrap();

        window
            .set_cursor_position(app_state.window_center())
            .expect("Failed to set cursor position!");

        Ok(())
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        renderer.render(
            app_state.window_dimensions.as_vec2(),
            self as &mut dyn Command,
        );

        Ok(())
    }
}

impl Command for DemoApp {
    fn issue_commands(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pipeline = self.pipeline.as_ref().expect("Failed to get pipeline!");

        self.model.buffers.bind(device, command_buffer);

        pipeline.bind(device, command_buffer);

        unsafe {
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline.layout(),
                0,
                &[self.pipeline_data.descriptor_set],
                &[],
            );

            device.cmd_draw_indexed(
                command_buffer,
                self.model.buffers.number_of_indices,
                1,
                0,
                0,
                1,
            );
        }

        Ok(())
    }

    fn recreate_pipelines(
        &mut self,
        context: Arc<VulkanContext>,
        swapchain: &VulkanSwapchain,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let descriptions = ObjModel::create_vertex_input_descriptions();
        let attributes = ObjModel::create_vertex_attributes();
        let vertex_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&descriptions)
            .vertex_attribute_descriptions(&attributes)
            .build();

        let shader_set = Arc::new(
            ShaderSet::new(context.clone())
                .context(CreateShaderSet {})?
                .vertex_shader("assets/shaders/model/model.vert.spv")
                .context(CreateShader {})?
                .fragment_shader("assets/shaders/model/model.frag.spv")
                .context(CreateShader {})?,
        );

        let descriptor_set_layout =
            Arc::new(ModelPipelineData::descriptor_set_layout(context.clone()));

        let settings = RenderPipelineSettings::new(
            swapchain.render_pass.clone(),
            vertex_state_info,
            descriptor_set_layout,
            shader_set,
        )
        .rasterization_samples(context.max_usable_samples());

        self.pipeline = None;
        self.pipeline = Some(RenderPipeline::new(context, settings));

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct UniformBufferObject {
    pub model: glm::Mat4,
    pub view: glm::Mat4,
    pub projection: glm::Mat4,
}

pub struct ModelPipelineData {
    pub descriptor_pool: DescriptorPool,
    pub uniform_buffer: Buffer,
    pub descriptor_set: vk::DescriptorSet,
    pub descriptor_set_layout: DescriptorSetLayout,
}

impl ModelPipelineData {
    pub fn new(context: Arc<VulkanContext>) -> Self {
        let descriptor_set_layout = Self::descriptor_set_layout(context.clone());
        let descriptor_pool = Self::create_descriptor_pool(context.clone());
        let descriptor_set = descriptor_pool
            .allocate_descriptor_sets(descriptor_set_layout.layout(), 1)
            .unwrap()[0];

        let uniform_buffer = Buffer::new_mapped_basic(
            context.clone(),
            mem::size_of::<UniformBufferObject>() as _,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk_mem::MemoryUsage::CpuToGpu,
        )
        .unwrap();

        let data = ModelPipelineData {
            descriptor_pool,
            uniform_buffer,
            descriptor_set,
            descriptor_set_layout,
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

        DescriptorSetLayout::new(context, layout_create_info).unwrap()
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

        DescriptorPool::new(context, pool_info).unwrap()
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
