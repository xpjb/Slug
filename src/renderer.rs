//! wgpu pipeline, bind groups, and render pass for Slug text rendering.

use wgpu::util::DeviceExt;
use crate::glyph_cache::GlyphCache;
use crate::vertex::{SlugVertex, create_text_vertices};
use glam::Mat4;

/// Uniform parameters for Slug: MVP matrix and viewport.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SlugParams {
    slug_matrix: [[f32; 4]; 4],
    slug_viewport: [f32; 4],
}

/// Slug text renderer using wgpu.
pub struct SlugRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    texture_bind_group: wgpu::BindGroup,
    curve_texture: wgpu::Texture,
    band_texture: wgpu::Texture,
    vertex_count: u32,
}

impl SlugRenderer {
    /// Create a new renderer.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
        cache: &GlyphCache,
        items: &[(&crate::glyph_cache::GlyphInfo, f32, f32)],
        color: [f32; 4],
    ) -> Self {
        let vertices = create_text_vertices(items, color);
        let vertex_count = vertices.len() as u32;

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slug vertex buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut params = SlugParams {
            slug_matrix: Mat4::IDENTITY.to_cols_array_2d(),
            slug_viewport: [config.width as f32, config.height as f32, 0.0, 0.0],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Slug uniform buffer"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let (curve_texture, band_texture) = Self::create_textures(device, queue, cache);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Slug shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/vertex.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Slug pipeline layout"),
            bind_group_layouts: &[&Self::uniform_bind_layout(device), &Self::texture_bind_layout(device)],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Slug pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<SlugVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 32,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 48,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                        wgpu::VertexAttribute {
                            offset: 64,
                            shader_location: 4,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("Slug fragment shader"),
                    source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                        "shaders/pixel.wgsl"
                    ))),
                }),
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            depth_stencil: None,
            multiview: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::uniform_bind_layout(device),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("Slug uniform bind group"),
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::texture_bind_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&curve_texture.create_view(&Default::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&band_texture.create_view(&Default::default())),
                },
            ],
            label: Some("Slug texture bind group"),
        });

        Self {
            pipeline,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            texture_bind_group,
            curve_texture,
            band_texture,
            vertex_count,
        }
    }

    fn uniform_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Slug uniform layout"),
        })
    }

    fn texture_bind_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
            label: Some("Slug texture layout"),
        })
    }

    fn create_textures(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cache: &GlyphCache,
    ) -> (wgpu::Texture, wgpu::Texture) {
        let (cw, ch) = cache.curve_size();
        let (bw, bh) = cache.band_size();

        // Pad curve data to full 4096-wide rows (glyph cache stores flat list)
        let curve_data = cache.curve_data();
        let curve_texels_per_row = cw as usize;
        let mut curve_padded = vec![[0.0f32; 4]; curve_texels_per_row * ch as usize];
        for (i, t) in curve_data.iter().enumerate() {
            if i < curve_padded.len() {
                curve_padded[i] = *t;
            }
        }
        let curve_bytes = bytemuck::cast_slice(&curve_padded);

        let curve_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Slug curve texture"),
            size: wgpu::Extent3d {
                width: cw,
                height: ch,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Pad band data to full 4096-wide rows
        let band_data = cache.band_data();
        let band_texels_per_row = bw as usize;
        let mut band_padded = vec![[0u32; 4]; band_texels_per_row * bh as usize];
        for (i, t) in band_data.iter().enumerate() {
            if i < band_padded.len() {
                band_padded[i] = *t;
            }
        }
        let band_bytes = bytemuck::cast_slice(&band_padded);

        let band_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Slug band texture"),
            size: wgpu::Extent3d {
                width: bw,
                height: bh,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &curve_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            curve_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(cw * 16),
                rows_per_image: Some(ch),
            },
            wgpu::Extent3d {
                width: cw,
                height: ch,
                depth_or_array_layers: 1,
            },
        );
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &band_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            band_bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bw * 16),
                rows_per_image: Some(bh),
            },
            wgpu::Extent3d {
                width: bw,
                height: bh,
                depth_or_array_layers: 1,
            },
        );

        (curve_texture, band_texture)
    }

    /// Update the MVP matrix and viewport, then record render commands.
    /// When `clear` is true, the target is cleared before drawing; when false, LoadOp::Load is used.
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        matrix: Mat4,
        viewport: (u32, u32),
        clear: bool,
    ) {
        let params = SlugParams {
            slug_matrix: matrix.to_cols_array_2d(),
            slug_viewport: [viewport.0 as f32, viewport.1 as f32, 0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&params));

        if self.vertex_count > 0 {
            let load_op = if clear {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.92,
                    g: 0.92,
                    b: 0.94,
                    a: 1.0,
                })
            } else {
                wgpu::LoadOp::Load
            };
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slug render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, &self.uniform_bind_group, &[]);
            rpass.set_bind_group(1, &self.texture_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..self.vertex_count, 0..1);
        }
    }
}
