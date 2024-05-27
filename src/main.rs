mod egui_tools;

use crate::egui_tools::EguiRenderer;
use egui_wgpu::{ScreenDescriptor};
use std::sync::Arc;
use wgpu::{Backends, InstanceDescriptor, TextureFormat};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        pollster::block_on(run());
    }
}

async fn run() {
    let event_loop = EventLoop::new().unwrap();

    let mut builder = winit::window::WindowBuilder::new();
    let window = builder.build(&event_loop).unwrap();
    let window = Arc::new(window);
    let initial_width = 1360;
    let initial_height = 768;
    window.request_inner_size(PhysicalSize::new(initial_width, initial_height));
    let instance = wgpu::Instance::new(InstanceDescriptor {
        backends: Backends::VULKAN,
        flags: Default::default(),
        dx12_shader_compiler: Default::default(),
        gles_minor_version: Default::default(),
    });
    let mut surface = unsafe { instance.create_surface(window.clone()) }.unwrap();
    let power_pref = wgpu::PowerPreference::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: power_pref,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find an appropriate adapter");

    let mut features = wgpu::Features::empty();
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    let selected_format = TextureFormat::Bgra8UnormSrgb;
    let swapchain_format = swapchain_capabilities
        .formats
        .iter()
        .find(|d| **d == selected_format)
        .expect("failed to select proper surface texture format!");

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: *swapchain_format,
        width: initial_width,
        height: initial_height,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 0,
        alpha_mode: swapchain_capabilities.alpha_modes[0],
        view_formats: vec![],
    };

    surface.configure(&device, &config);

    let mut egui_renderer = EguiRenderer::new(&device, config.format, None, 1, &window);

    let mut close_requested = false;
    let mut modifiers = ModifiersState::default();

    let mut scale_factor = 1.0;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent { event, .. } => {
                egui_renderer.handle_input(&window, &event);

                match event {
                    WindowEvent::CloseRequested => {
                        close_requested = true;
                    }
                    WindowEvent::ModifiersChanged(new) => {
                        modifiers = new.state();
                    }
                    WindowEvent::KeyboardInput {
                        event: kb_event, ..
                    } => {
                        if kb_event.logical_key == Key::Named(NamedKey::Escape) {
                            close_requested = true;
                            return;
                        }
                    }
                    WindowEvent::ActivationTokenDone { .. } => {}
                    WindowEvent::Resized(new_size) => {
                        // Resize surface:
                        config.width = new_size.width;
                        config.height = new_size.height;
                        surface.configure(&device, &config);
                    }
                    WindowEvent::Moved(_) => {}
                    WindowEvent::Destroyed => {}
                    WindowEvent::DroppedFile(_) => {}
                    WindowEvent::HoveredFile(_) => {}
                    WindowEvent::HoveredFileCancelled => {}
                    WindowEvent::Focused(_) => {}
                    WindowEvent::Ime(_) => {}
                    WindowEvent::CursorMoved { .. } => {}
                    WindowEvent::CursorEntered { .. } => {}
                    WindowEvent::CursorLeft { .. } => {}
                    WindowEvent::MouseWheel { .. } => {}
                    WindowEvent::MouseInput { .. } => {}
                    WindowEvent::TouchpadMagnify { .. } => {}
                    WindowEvent::SmartMagnify { .. } => {}
                    WindowEvent::TouchpadRotate { .. } => {}
                    WindowEvent::TouchpadPressure { .. } => {}
                    WindowEvent::AxisMotion { .. } => {}
                    WindowEvent::Touch(_) => {}
                    WindowEvent::ScaleFactorChanged { .. } => {}
                    WindowEvent::ThemeChanged(_) => {}
                    WindowEvent::Occluded(_) => {}
                    WindowEvent::RedrawRequested => {
                        let surface_texture = surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture");

                        let surface_view = surface_texture
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: None,
                            });

                        let screen_descriptor = ScreenDescriptor {
                            size_in_pixels: [config.width, config.height],
                            pixels_per_point: window.scale_factor() as f32 * scale_factor,
                        };

                        egui_renderer.draw(
                            &device,
                            &queue,
                            &mut encoder,
                            &window,
                            &surface_view,
                            screen_descriptor,
                            |ctx| {
                                egui::Window::new("winit + egui + wgpu says hello!")
                                    .resizable(true)
                                    .vscroll(true)
                                    .default_open(false)
                                    .show(&ctx, |mut ui| {
                                        ui.label("Label!");

                                        if ui.button("Button!").clicked() {
                                            println!("boom!")
                                        }

                                        ui.separator();
                                        ui.horizontal(|ui| {
                                            ui.label(format!(
                                                "Pixels per point: {}",
                                                ctx.pixels_per_point()
                                            ));
                                            if ui.button("-").clicked() {
                                                scale_factor = (scale_factor - 0.1).max(0.3);
                                            }
                                            if ui.button("+").clicked() {
                                                scale_factor = (scale_factor + 0.1).min(3.0);
                                            }
                                        });
                                    });
                            },
                        );

                        queue.submit(Some(encoder.finish()));
                        surface_texture.present();
                        window.request_redraw();
                    }
                }
            }

            Event::NewEvents(_) => {}
            Event::DeviceEvent { .. } => {}
            Event::UserEvent(_) => {}
            Event::Suspended => {}
            Event::Resumed => {}
            Event::AboutToWait => {
                if close_requested {
                    elwt.exit()
                }
            }
            Event::LoopExiting => {}
            Event::MemoryWarning => {}
        }
    });
}
