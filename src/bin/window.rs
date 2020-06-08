use std::{boxed::Box, error::Error};
use support::{
    app::{run_app, setup_app, App, AppState},
    vulkan::{Command, Renderer},
};
use winit::window::Window;

#[derive(Default)]
struct DemoApp;

impl App for DemoApp {
    fn initialize(
        &mut self,
        _: &mut Window,
        renderer: &mut Renderer,
        _: &AppState,
    ) -> Result<(), Box<dyn Error>> {
        renderer.record_all_command_buffers(self as &mut dyn Command);
        Ok(())
    }

    fn draw(
        &mut self,
        renderer: &mut Renderer,
        app_state: &AppState,
    ) -> Result<(), Box<dyn Error>> {
        renderer.render(
            app_state.window_dimensions.as_vec2(),
            self as &mut dyn Command,
        );
        Ok(())
    }
}

impl Command for DemoApp {}

fn main() {
    let (window, event_loop, renderer) = setup_app("Vulkan Window");
    run_app(DemoApp::default(), window, event_loop, renderer);
}
