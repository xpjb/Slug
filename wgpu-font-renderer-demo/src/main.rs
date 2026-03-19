//! wgpu-font-renderer demo - renders "Hello 你好 日本語" using Silver.ttf and NotoSansSC-Regular.ttf.
//! Same layout as slug-demo: two lines, Silver at (50,150), Noto at (50,390), font size 200.

use pollster::block_on;
use std::path::PathBuf;
use std::rc::Rc;
use wgpu_font_renderer::{FontStore, TextRenderer, TypeWriter};

const FONT_SIZE: f32 = 200.0;
const LINE_SPACING_EM: f32 = 1.2;
const TEXT: &str = "Hello 你好 日本語";

fn main() {
    block_on(run());
}

async fn run() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("..");

    let silver_path = workspace_root.join("Silver.ttf");
    let noto_path = workspace_root.join("NotoSansSC-Regular.ttf");

    let (window, event_loop, size) = {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = winit::window::WindowBuilder::new()
            .with_title("wgpu-font-renderer - Hello")
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
            .build(&event_loop)
            .unwrap();
        let size = window.inner_size();
        (Rc::new(window), event_loop, size)
    };

    let window_ref = window.clone();
    let instance = wgpu::Instance::default();
    let surface = instance.create_surface(window_ref.as_ref()).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Failed to find adapter");
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .expect("Failed to create device");

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // Preset: characters to cache - include our text
    let preset = "Hello 你好 日本語 abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    let mut font_store = FontStore::new(&device, &config);
    let silver_key = font_store
        .load(
            &device,
            &queue,
            silver_path.to_str().expect("Silver path"),
            preset,
        )
        .expect("Failed to load Silver.ttf");
    let noto_key = font_store
        .load(
            &device,
            &queue,
            noto_path.to_str().expect("Noto path"),
            preset,
        )
        .expect("Failed to load NotoSansSC-Regular.ttf");

    let silver_y = 150.0;
    let noto_y = silver_y + FONT_SIZE * LINE_SPACING_EM;

    let color = [0.1, 0.1, 0.1, 1.0];

    let mut type_writer = TypeWriter::new();
    let mut paragraphs = Vec::new();
    if let Some(p) = type_writer.shape_text(
        &font_store,
        silver_key,
        [50.0, silver_y],
        FONT_SIZE as u16,
        color,
        TEXT,
    ) {
        paragraphs.push(p);
    }
    if let Some(p) = type_writer.shape_text(
        &font_store,
        noto_key,
        [50.0, noto_y],
        FONT_SIZE as u16,
        color,
        TEXT,
    ) {
        paragraphs.push(p);
    }

    let mut text_renderer = TextRenderer::new(&device, &config, font_store.atlas());

    window.request_redraw();

    event_loop
        .run(move |event, elwt| {
            match event {
                winit::event::Event::WindowEvent { window_id: _, event } => match event {
                    winit::event::WindowEvent::CloseRequested => elwt.exit(),
                    winit::event::WindowEvent::Resized(physical_size) => {
                        surface.configure(&device, &wgpu::SurfaceConfiguration {
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                            width: physical_size.width,
                            height: physical_size.height,
                            present_mode: wgpu::PresentMode::Fifo,
                            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                            view_formats: vec![],
                            desired_maximum_frame_latency: 2,
                        });
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        let current_size = window.inner_size();
                        if current_size.width > 0 && current_size.height > 0 {
                            if let Ok(frame) = surface.get_current_texture() {
                                let view = frame.texture.create_view(&Default::default());
                                let mut encoder = device
                                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                                text_renderer.prepare(&device, &paragraphs, &font_store);

                                {
                                    let mut rpass = encoder.begin_render_pass(
                                        &wgpu::RenderPassDescriptor {
                                            label: Some("Text render pass"),
                                            color_attachments: &[Some(
                                                wgpu::RenderPassColorAttachment {
                                                    view: &view,
                                                    resolve_target: None,
                                                    ops: wgpu::Operations {
                                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                                            r: 0.92,
                                                            g: 0.92,
                                                            b: 0.94,
                                                            a: 1.0,
                                                        }),
                                                        store: wgpu::StoreOp::Store,
                                                    },
                                                },
                                            )],
                                            depth_stencil_attachment: None,
                                            timestamp_writes: None,
                                            occlusion_query_set: None,
                                        },
                                    );
                                    text_renderer.render(
                                        &mut rpass,
                                        [current_size.width, current_size.height],
                                    );
                                }

                                queue.submit(std::iter::once(encoder.finish()));
                                frame.present();
                            }
                        }
                    }
                    _ => {}
                },
                winit::event::Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}
