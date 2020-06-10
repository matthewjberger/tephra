use ash::vk;
use log::debug;
use nalgebra_glm as glm;
use std::{boxed::Box, sync::Arc};
use support::{
    app::{run_app, setup_app, App, AppState},
    camera::FreeCamera,
    vulkan::{
        create_skybox_pipeline, Command, HdrCubemap, RenderPass, RenderPipeline, Renderer,
        ShaderCache, SkyboxPipelineData, SkyboxRenderer, SkyboxUniformBufferObject, VulkanContext,
    },
};
use winit::window::Window;

fn main() {
    let (window, event_loop, renderer) = setup_app("Physically Based Rendering - Gltf models");
    run_app(
        DemoApp::new(renderer.context.clone()),
        window,
        event_loop,
        renderer,
    );
}

struct DemoApp {
    context: Arc<VulkanContext>,
    skybox_pipeline: Option<RenderPipeline>,
    skybox_pipeline_data: Option<SkyboxPipelineData>,
    camera: FreeCamera,
}

impl DemoApp {
    pub fn new(context: Arc<VulkanContext>) -> Self {
        Self {
            context,
            skybox_pipeline: None,
            skybox_pipeline_data: None,
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
        window.set_cursor_visible(false);
        window
            .set_cursor_grab(true)
            .expect("Failed to grab cursor!");

        self.camera.position_at(&glm::vec3(0.0, -4.0, -4.0));
        self.camera.look_at(&glm::vec3(0.0, 0.0, 0.0));

        window.set_cursor_position(app_state.window_center())?;

        let cubemap_path = "assets/skyboxes/walk_of_fame/walk_of_fame.hdr";

        debug!("Creating HDR cubemap");
        let hdr = HdrCubemap::new(
            self.context.clone(),
            &renderer.transient_command_pool,
            &cubemap_path,
            &mut renderer.shader_cache,
        )?;

        let skybox_pipeline_data = SkyboxPipelineData::new(
            self.context.clone(),
            &renderer.transient_command_pool,
            &hdr.cubemap,
        );

        self.skybox_pipeline_data = Some(skybox_pipeline_data);

        let render_pass = renderer.vulkan_swapchain().render_pass.clone();
        self.recreate_pipelines(
            renderer.context.clone(),
            &mut renderer.shader_cache,
            render_pass,
        )?;

        renderer.record_all_command_buffers(self as &mut dyn Command);

        Ok(())
    }

    fn update(
        &mut self,
        window: &mut Window,
        renderer: &mut Renderer,
        app_state: &AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.camera.update(&app_state);

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

        let view = self.camera.view_matrix();

        if let Some(skybox_data) = &self.skybox_pipeline_data.as_ref() {
            let skybox_ubo = SkyboxUniformBufferObject { view, projection };
            let skybox_ubos = [skybox_ubo];

            skybox_data
                .uniform_buffer
                .upload_to_buffer(&skybox_ubos, 0)?;
        }

        window.set_cursor_position(app_state.window_center())?;

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
        let skybox_pipeline = self
            .skybox_pipeline
            .as_ref()
            .expect("Failed to get skybox pipeline!");

        let skybox_pipeline_data = self
            .skybox_pipeline_data
            .as_ref()
            .expect("Failed to get skybox pipeline data!");

        skybox_pipeline.bind(device, command_buffer);

        let skybox_renderer =
            SkyboxRenderer::new(command_buffer, &skybox_pipeline, &skybox_pipeline_data);

        skybox_renderer.draw(device, &skybox_pipeline_data.cube);

        Ok(())
    }

    fn recreate_pipelines(
        &mut self,
        context: Arc<VulkanContext>,
        shader_cache: &mut ShaderCache,
        render_pass: Arc<RenderPass>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.skybox_pipeline = None;
        self.skybox_pipeline = Some(create_skybox_pipeline(context, shader_cache, render_pass));

        Ok(())
    }
}
