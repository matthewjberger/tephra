use crate::{
    input::Input,
    vulkan::{Renderer, VulkanContext},
};
use nalgebra_glm as glm;
use std::{sync::Arc, time::Instant};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[derive(Default)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn as_vec2(&self) -> glm::Vec2 {
        glm::vec2(self.width as _, self.height as _)
    }
}

#[derive(Default)]
pub struct AppState {
    pub window_dimensions: Dimensions,
    pub input: Input,
    pub delta_time: f64,
}

impl AppState {
    pub fn window_center(&self) -> PhysicalPosition<i32> {
        PhysicalPosition::new(
            (self.window_dimensions.width as f32 / 2.0) as i32,
            (self.window_dimensions.height as f32 / 2.0) as i32,
        )
    }
}

pub trait App {
    fn initialize(&mut self, _: &mut Window, _: &mut Renderer, _: &AppState) {}
    fn update(&mut self, _: &mut Window, _: &mut Renderer, _: &AppState) {}
    fn draw(&mut self, _: &mut Renderer, _: &AppState) {}
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
    mut window: Window,
    event_loop: EventLoop<()>,
    mut renderer: Renderer,
) where
    T: App,
{
    let mut app_state = AppState::default();
    let window_size = window.inner_size();
    app_state.window_dimensions = Dimensions::new(window_size.width, window_size.height);

    renderer.allocate_command_buffers();

    app.initialize(&mut window, &mut renderer, &app_state);

    let mut last_frame = Instant::now();
    let mut cursor_moved = false;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents { .. } => {
                app_state.delta_time =
                    (Instant::now().duration_since(last_frame).as_micros() as f64) / 1_000_000_f64;
                last_frame = Instant::now();
                app.update(&mut window, &mut renderer, &app_state);
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
                    *app_state.input.keystates.entry(keycode).or_insert(state) = state;
                }
                WindowEvent::Resized(PhysicalSize { width, height }) => {
                    app_state.window_dimensions = Dimensions::new(width, height);
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    let clicked = state == ElementState::Pressed;
                    match button {
                        MouseButton::Left => app_state.input.mouse.is_left_clicked = clicked,
                        MouseButton::Right => app_state.input.mouse.is_right_clicked = clicked,
                        _ => {}
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let last_position = app_state.input.mouse.position;
                    let current_position = glm::vec2(position.x as _, position.y as _);
                    app_state.input.mouse.position = current_position;
                    app_state.input.mouse.position_delta = current_position - last_position;
                    let center = app_state.window_center();
                    app_state.input.mouse.offset_from_center = glm::vec2(
                        (center.x - position.x as i32) as _,
                        (center.y - position.y as i32) as _,
                    );
                    cursor_moved = true;
                }
                WindowEvent::MouseWheel {
                    delta: MouseScrollDelta::LineDelta(_, v_lines),
                    ..
                } => {
                    app_state.input.mouse.wheel_delta = v_lines;
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                if !cursor_moved {
                    app_state.input.mouse.position_delta = glm::vec2(0.0, 0.0);
                }
                cursor_moved = false;

                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                app.draw(&mut renderer, &app_state);
            }
            Event::RedrawEventsCleared => {
                app_state.input.mouse.wheel_delta = 0.0;
            }
            _ => {}
        }
    });
}
