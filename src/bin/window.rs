use ash::vk;
use nalgebra_glm as glm;
use support::{
    app::{run_app, App},
    vulkan::{Command, Renderer},
};

#[derive(Default)]
struct DemoApp;

impl App for DemoApp {
    fn initialize(&mut self, renderer: &mut Renderer) {
        renderer.record_all_command_buffers(self as &mut dyn Command);
    }

    fn draw(&mut self, renderer: &mut Renderer, window_dimensions: glm::Vec2) {
        renderer.render(window_dimensions, self as &mut dyn Command);
    }
}

impl Command for DemoApp {
    fn issue_commands(&mut self, _: vk::CommandBuffer) {}
}

fn main() {
    run_app(DemoApp::default(), "Vulkan Window");
}