use nalgebra_glm as glm;
use support::{
    app::{run_app, App},
    vulkan::Renderer,
};

#[derive(Default)]
struct DemoApp;

impl App for DemoApp {
    fn initialize(&mut self, renderer: &mut Renderer) {
        renderer.record_all_command_buffers(&|_| {});
    }

    fn draw(&mut self, renderer: &mut Renderer, window_dimensions: glm::Vec2) {
        renderer.render(window_dimensions, &|_| {});
    }
}

fn main() {
    run_app(DemoApp::default(), "Vulkan Window");
}
