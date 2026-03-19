//! Slug font rendering demo - renders "Hello" using Silver.ttf and NotoSansCJKsc-Regular.ttf.
//!
//! Set SLUG_DEBUG=1 to print debug info and exit without rendering.

use pollster::block_on;
use slug::{FontLoader, fonts_in_collection, pick_ttc_face_index, GlyphCache, GlyphInfo, SlugRenderer, create_text_vertices, layout_text};
use glam::{Mat4, Vec4};
use std::path::PathBuf;
use std::rc::Rc;

const FONT_SIZE: f32 = 200.0;
const LINE_SPACING_EM: f32 = 1.2;

fn main() {
    block_on(run());
}

async fn run() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Load Silver font (may be TTC/OTC: probe for correct face to avoid LSB-as-advance)
    let mut silver_path = manifest_dir.clone();
    silver_path.push("Silver.ttf");
    let silver_bytes = std::fs::read(&silver_path).expect("Failed to read Silver.ttf");
    let silver_face_index = match fonts_in_collection(&silver_bytes) {
        Some(n) if n > 1 => pick_ttc_face_index(&silver_bytes, n),
        _ => 0,
    };
    let silver_loader = FontLoader::from_bytes_with_index(silver_bytes, silver_face_index)
        .expect("Failed to parse Silver.ttf");

    // Load Noto CJK font (may be TTC/OTC: use correct face index for SC)
    let mut noto_path = manifest_dir;
    noto_path.push("NotoSansCJKsc-Regular.ttf");
    let noto_bytes = std::fs::read(&noto_path).expect("Failed to read NotoSansCJKsc-Regular.ttf");
    let noto_face_index = match fonts_in_collection(&noto_bytes) {
        Some(n) if n > 1 => {
            // TTC/OTC: probe each face to find one with correct hmtx (avoids LSB-as-advance bug)
            pick_ttc_face_index(&noto_bytes, n)
        }
        _ => 0,
    };
    let noto_loader = FontLoader::from_bytes_with_index(noto_bytes, noto_face_index)
        .expect("Failed to parse NotoSansCJKsc-Regular.ttf");

    let mut silver_cache = GlyphCache::new();
    let mut noto_cache = GlyphCache::new();

    let text = "Hello 你好 日本語";  // Latin, Chinese, Japanese
    let silver_items = layout_text(&silver_loader, &mut silver_cache, text, 0.0, 0.0);
    let noto_items = layout_text(&noto_loader, &mut noto_cache, text, 0.0, 0.0);

    if silver_items.is_empty() && noto_items.is_empty() {
        eprintln!("No glyphs to render");
        return;
    }

    let color = [0.1, 0.1, 0.1, 1.0];
    let silver_items_ref: Vec<_> = silver_items.iter().map(|(info, x, y)| (info, *x, *y)).collect();
    let noto_items_ref: Vec<_> = noto_items.iter().map(|(info, x, y)| (info, *x, *y)).collect();

    // Debug mode: print diagnostics and exit (set SLUG_DEBUG=1)
    if std::env::var("SLUG_DEBUG").as_deref() == Ok("1") {
        let silver_upem = silver_loader.units_per_em() as f32;
        println!("=== SILVER FONT ===");
        debug_print(&silver_cache, &silver_items_ref, color, silver_upem);
        println!("\n=== NOTO FONT ===");
        let noto_upem = noto_loader.units_per_em() as f32;
        debug_print(&noto_cache, &noto_items_ref, color, noto_upem);
        return;
    }

    let (window, event_loop, size) = {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = winit::window::WindowBuilder::new()
            .with_title("Slug - Hello")
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

    let renderer_silver = SlugRenderer::new(&device, &queue, &config, &silver_cache, &silver_items_ref, color);
    let renderer_noto = SlugRenderer::new(&device, &queue, &config, &noto_cache, &noto_items_ref, color);

    let silver_y = 150.0;
    let noto_y = silver_y + FONT_SIZE * LINE_SPACING_EM; // 150 + 240 = 390

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
                                let mut encoder =
                                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                                let proj = Mat4::orthographic_rh(
                                    0.0,
                                    current_size.width as f32,
                                    current_size.height as f32,
                                    0.0,
                                    -1.0,
                                    1.0,
                                );

                                // Silver: first line at (50, 150)
                                let scale_silver = FONT_SIZE;
                                let view_silver = Mat4::from_translation(glam::Vec3::new(50.0, silver_y, 0.0));
                                let model_silver = Mat4::from_scale(glam::Vec3::new(scale_silver, -scale_silver, 1.0));
                                let matrix_silver = proj * view_silver * model_silver;

                                // Noto: second line at (50, noto_y)
                                let scale_noto = FONT_SIZE;
                                let view_noto = Mat4::from_translation(glam::Vec3::new(50.0, noto_y, 0.0));
                                let model_noto = Mat4::from_scale(glam::Vec3::new(scale_noto, -scale_noto, 1.0));
                                let matrix_noto = proj * view_noto * model_noto;

                                renderer_silver.render(
                                    &queue,
                                    &mut encoder,
                                    &view,
                                    matrix_silver,
                                    (current_size.width, current_size.height),
                                    true,
                                );
                                renderer_noto.render(
                                    &queue,
                                    &mut encoder,
                                    &view,
                                    matrix_noto,
                                    (current_size.width, current_size.height),
                                    false,
                                );

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

fn debug_print(
    cache: &GlyphCache,
    items: &[(&GlyphInfo, f32, f32)],
    color: [f32; 4],
    upem: f32,
) {
    let vertices = create_text_vertices(items, color);
    let (cw, ch) = cache.curve_size();
    let (bw, bh) = cache.band_size();
    let curve_data = cache.curve_data();
    let band_data = cache.band_data();

    println!("=== SLUG DEBUG ===\n");
    println!("Glyphs: {}  Vertices: {}", items.len(), vertices.len());
    println!("Curve texture: {}x{}  Band texture: {}x{}\n", cw, ch, bw, bh);

    for (i, (info, px, py)) in items.iter().enumerate() {
        println!("--- Glyph {} (pos {:.4}, {:.4}) ---", i, px, py);
        println!("  curve_start: {:?}  band_start: {:?}  band_max: {:?}", info.curve_start, info.band_start, info.band_max);
        println!("  bbox: ({:.4}, {:.4}, {:.4}, {:.4})\n", info.bbox.0, info.bbox.1, info.bbox.2, info.bbox.3);
    }

    if let Some(v) = vertices.first() {
        println!("--- First vertex (pos) ---");
        println!("  pos: {:?}", v.pos);
        println!("  tex: {:?} (tex.zw bits: {:08X} {:08X})", v.tex, v.tex[2].to_bits(), v.tex[3].to_bits());
        println!("  bnd: {:?}\n", v.bnd);
    }

    let size_w = 800.0f32;
    let size_h = 600.0f32;
    let font_size = 200.0;
    let scale = font_size / upem;
    let proj = Mat4::orthographic_rh(0.0, size_w, size_h, 0.0, -1.0, 1.0);
    let view = Mat4::from_translation(glam::Vec3::new(50.0, 150.0, 0.0));
    let model = Mat4::from_scale(glam::Vec3::new(scale, -scale, 1.0));
    let matrix = proj * view * model;

    println!("--- Clip-space bounds (first glyph corners) ---");
    for (_i, (info, px, py)) in items.iter().enumerate().take(1) {
        let (min_x, min_y, max_x, max_y) = info.bbox;
        let corners = [
            (px + min_x, py + min_y),
            (px + max_x, py + min_y),
            (px + max_x, py + max_y),
            (px + min_x, py + max_y),
        ];
        for (j, (ex, ey)) in corners.iter().enumerate() {
            let v = matrix * Vec4::new(*ex, *ey, 0.0, 1.0);
            let ndc = (v.x / v.w, v.y / v.w);
            let in_view = ndc.0 >= -1.0 && ndc.0 <= 1.0 && ndc.1 >= -1.0 && ndc.1 <= 1.0;
            println!("  corner {}: em=({:.2},{:.2}) clip=({:.3},{:.3}) in_view={}", j, ex, ey, ndc.0, ndc.1, in_view);
        }
    }

    println!("\n--- Curve texels (first 4) ---");
    for (i, t) in curve_data.iter().take(4).enumerate() {
        println!("  [{}] {:?}", i, t);
    }

    println!("\n--- Band texels (first 8) ---");
    for (i, t) in band_data.iter().take(8).enumerate() {
        println!("  [{}] {:?}", i, t);
    }

    println!("\n=== END DEBUG (exiting) ===");
}
