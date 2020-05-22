use nalgebra_glm as glm;
use std::{time::Instant, sync::Arc};
use ash::vk;
use winit::{
    dpi::PhysicalSize,
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
};
use crate::vulkan::{VulkanContext, SynchronizationSet, CommandPool, VulkanSwapchain};

pub trait App {
    fn initialize(&mut self, _: Arc<VulkanContext>) {}
    fn update(&mut self, _: Arc<VulkanContext>, _: f64) {}
    fn render(&mut self, _: Arc<VulkanContext>) {}
    fn cleanup(&mut self) {}
    fn handle_resize(&mut self, _: u32, _: u32) {}
    fn handle_key_pressed(&mut self, _: VirtualKeyCode, _: ElementState) {}
    fn handle_mouse_clicked(&mut self, _: MouseButton, _: ElementState) {}
    fn handle_mouse_scrolled(&mut self, _: f32) {}
    fn handle_cursor_moved(&mut self, _: glm::Vec2) {}
}

pub fn run_app<T: 'static>(mut app: T, title: &str)
where
    T: App,
{
    let (width, height) = (1920, 1080);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(title)
        .with_inner_size(PhysicalSize::new(width, height))
        .build(&event_loop)
        .expect("Failed to create window.");

    let vulkan_context = Arc::new(VulkanContext::new(&window).expect("Failed to create a vulkan context!"));

    let synchronization_set =
        SynchronizationSet::new(vulkan_context.clone()).expect("Failed to create sync objects");

    let command_pool = CommandPool::new(
        vulkan_context.clone(),
        vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    );

    let transient_command_pool =
        CommandPool::new(vulkan_context.clone(), vk::CommandPoolCreateFlags::TRANSIENT);

    let logical_size = window.inner_size();
    let dimensions = [logical_size.width as u32, logical_size.height as u32];

    let vulkan_swapchain = Some(VulkanSwapchain::new(
        vulkan_context.clone(),
        dimensions,
        &command_pool,
    ));

    app.initialize(vulkan_context.clone());

    let mut last_frame = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents { .. } => {
                let delta_time =
                    (Instant::now().duration_since(last_frame).as_micros() as f64) / 1_000_000_f64;
                last_frame = Instant::now();
                app.update(vulkan_context.clone(), delta_time);
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
                    app.handle_resize(width, height)
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
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                app.render(vulkan_context.clone());
            }
            _ => {}
        }
    });
}
