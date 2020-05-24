use crate::vulkan::{Renderer, VulkanContext};
use nalgebra_glm as glm;
use std::{sync::Arc, time::Instant};
use winit::{
    dpi::PhysicalSize,
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub trait App {
    fn initialize(&mut self, _: &mut Renderer) {}
    fn update(&mut self, _: &mut Renderer, _: f64) {}
    fn draw(&mut self, _: &mut Renderer, _: glm::Vec2) {}
    fn handle_resize(&mut self, _: u32, _: u32) {}
    fn handle_key_pressed(&mut self, _: VirtualKeyCode, _: ElementState) {}
    fn handle_mouse_clicked(&mut self, _: MouseButton, _: ElementState) {}
    fn handle_mouse_scrolled(&mut self, _: f32) {}
    fn handle_cursor_moved(&mut self, _: glm::Vec2) {}
}

pub fn setup_app(title: &str) -> (Window, EventLoop<()>, Renderer) {
    let (width, height) = (1920, 1080);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(width, height))
        .build(&event_loop)
        .expect("Failed to create window.");

    let vulkan_context =
        Arc::new(VulkanContext::new(&window).expect("Failed to create a vulkan context!"));

    let renderer = Renderer::new(vulkan_context, &window);

    (window, event_loop, renderer)
}

pub fn run_app<T: 'static>(
    mut app: T,
    window: Window,
    event_loop: EventLoop<()>,
    mut renderer: Renderer,
) where
    T: App,
{
    renderer.allocate_command_buffers();

    app.initialize(&mut renderer);

    let mut last_frame = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents { .. } => {
                let delta_time =
                    (Instant::now().duration_since(last_frame).as_micros() as f64) / 1_000_000_f64;
                last_frame = Instant::now();
                app.update(&mut renderer, delta_time);
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            state,
                            ..
                        },
                    ..
                } => {
                    if keycode == VirtualKeyCode::Escape {
                        *control_flow = ControlFlow::Exit;
                    }

                    app.handle_key_pressed(keycode, state);
                }
                WindowEvent::Resized(PhysicalSize { width, height }) => {
                    app.handle_resize(width, height);
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    app.handle_mouse_clicked(button, state);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    app.handle_cursor_moved(glm::vec2(position.x as _, position.y as _));
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, v_lines),
                    ..
                } => {
                    app.handle_mouse_scrolled(v_lines);
                }
                _ => {}
            },
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawRequested(_) => {
                let window_inner_size = window.inner_size();
                let window_size = glm::vec2(
                    window_inner_size.width as f32,
                    window_inner_size.height as f32,
                );
                app.draw(&mut renderer, window_size);
            }
            _ => {}
        }
    });
}
