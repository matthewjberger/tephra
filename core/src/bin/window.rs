use nalgebra_glm as glm;
use support::{
    app::{run_app, setup_app, App, AppState},
    vulkan::{Command, Renderer},
};
use winit::window::Window;

#[derive(Default)]
struct DemoApp;

impl App for DemoApp {
    fn initialize(&mut self, _: &mut Window, renderer: &mut Renderer, _: &AppState) {
        renderer.record_all_command_buffers(self as &mut dyn Command);
    }

    fn draw(&mut self, renderer: &mut Renderer, app_state: &AppState) {
        renderer.render(
            glm::vec2(
                app_state.window_dimensions.width as f32,
                app_state.window_dimensions.height as f32,
            ),
            self as &mut dyn Command,
        );
    }
}

impl Command for DemoApp {}

fn main() {
    let (window, event_loop, renderer) = setup_app("Vulkan Window");
    run_app(DemoApp::default(), window, event_loop, renderer);
}
