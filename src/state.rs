use std::sync::Arc;
use wgpu::{RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline};
use winit::window::Window;
use crate::{
    buffers::{Buffers, BindGroup, BindGroupEntry},
    vertex::Vertex,
    fluid::Fluid,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AspectRatio {
    ratio: f32,
}

const GRID_SIZE: usize = 64;

/// wgpuの全コンポーネントを保持し、レンダリングフローを制御するメイン構造体
pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: RenderPipeline,
    wireframe_pipeline: RenderPipeline,
    pub window: Arc<Window>,
    buffers: Buffers,
    
    fluid: Fluid,
    mouse_pressed: bool,
    mouse_pos: nalgebra_glm::Vec2,
    aspect_bind_group: wgpu::BindGroup,
    density_texture: wgpu::Texture,
    density_bind_group: wgpu::BindGroup,
    #[cfg(feature = "gui")]
    pub gui: crate::gui::Gui,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("適切なグラフィックスアダプターが見つかりませんでした。");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary Device"),
                    required_features: wgpu::Features::POLYGON_MODE_LINE,
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("GPUデバイスの作成に失敗しました。");

        let caps = surface.get_capabilities(&adapter);
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap_or_else(|| {
                wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: caps.formats[0],
                    width: size.width.max(1),
                    height: size.height.max(1),
                    present_mode: caps.present_modes[0],
                    alpha_mode: caps.alpha_modes[0],
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                }
            });

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // 単一の四角形（4頂点）を生成
        let (vertices, indices) = crate::vertex::create_quad();

        // アスペクト比のバインドグループ作成
        let initial_aspect = AspectRatio { ratio: size.width as f32 / size.height as f32 };
        let aspect_entry = BindGroupEntry::uniform(&device, initial_aspect);
        let aspect_bg_def = BindGroup::new("Aspect Bind Group").insert(aspect_entry);
        let aspect_layout = aspect_bg_def.bind_group_layout(&device);
        let aspect_bind_group = aspect_bg_def.bind_group(&device, &aspect_layout);

        // テクスチャの作成
        let texture_size = wgpu::Extent3d {
            width: GRID_SIZE as u32,
            height: GRID_SIZE as u32,
            depth_or_array_layers: 1,
        };
        let density_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Density Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let density_view = density_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let density_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let density_bg_def = BindGroup::new("Density Bind Group")
            .insert(BindGroupEntry::texture(density_view))
            .insert(BindGroupEntry::sampler(density_sampler));
        let density_layout = density_bg_def.bind_group_layout(&device);
        let density_bind_group = density_bg_def.bind_group(&device, &density_layout);
        
        let mut buffers = Buffers::new(&device, &vertices, &indices);
        
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&aspect_layout, &density_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // 両面表示
                polygon_mode: wgpu::PolygonMode::Fill,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let wireframe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Wireframe Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Line,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        #[cfg(feature = "gui")]
        let gui = crate::gui::Gui::new(&window, &device, config.format);

        // BindGroupの実体を管理するためにbuffersを少し整理
        buffers = buffers.add_bind_group(aspect_bg_def);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            wireframe_pipeline,
            window,
            buffers,
            fluid: Fluid::new(GRID_SIZE, 0.1, 0.0, 0.000001),
            mouse_pressed: false,
            mouse_pos: nalgebra_glm::Vec2::zeros(),
            aspect_bind_group,
            density_texture,
            density_bind_group,
            #[cfg(feature = "gui")]
            gui,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            
            // アスペクト比を更新
            let aspect = AspectRatio { ratio: new_size.width as f32 / new_size.height as f32 };
            self.buffers.bind_groups[0].entries[0].update_buffer(&self.queue, aspect);
        }
    }

    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        match event {
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                if *button == winit::event::MouseButton::Left {
                    self.mouse_pressed = *state == winit::event::ElementState::Pressed;
                }
                true
            }
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = nalgebra_glm::vec2(position.x as f32, position.y as f32);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self) {
        if self.mouse_pressed {
            let aspect = self.size.width as f32 / self.size.height as f32;

            // マウス座標を正規化 (0.0 - 1.0)
            let nx = self.mouse_pos.x / self.size.width as f32;
            let ny = self.mouse_pos.y / self.size.height as f32;

            // シェーダーの vs_main と同じロジックで正規化座標 (-1.0 - 1.0) に変換
            let (px, py) = if aspect > 1.0 {
                ((nx * 2.0 - 1.0) * aspect, ny * 2.0 - 1.0)
            } else {
                (nx * 2.0 - 1.0, (ny * 2.0 - 1.0) / aspect)
            };

            // -1.0 - 1.0 の空間を 0 - GRID_SIZE のインデックスに変換
            let x = (((px + 1.0) / 2.0) * GRID_SIZE as f32) as usize;
            let y = (((py + 1.0) / 2.0) * GRID_SIZE as f32) as usize;

            if x < GRID_SIZE && y < GRID_SIZE {
                self.fluid.add_density(x, y, 10.0);
            }
        }

        self.fluid.step();

        // 流体の密度（Vec<f32>）をテクスチャ（RGBA）に変換
        let mut data = vec![0u8; GRID_SIZE * GRID_SIZE * 4];
        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                let d = self.fluid.get_density(x, y);
                let idx = (y * GRID_SIZE + x) * 4;
                data[idx] = (d * 255.0).min(255.0) as u8;     // R
                data[idx + 1] = (d * 255.0).min(255.0) as u8; // G
                data[idx + 2] = (d * 255.0).min(255.0) as u8;       // B
                data[idx + 3] = 255;                                // A
            }
        }

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.density_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * GRID_SIZE as u32),
                rows_per_image: Some(GRID_SIZE as u32),
            },
            wgpu::Extent3d {
                width: GRID_SIZE as u32,
                height: GRID_SIZE as u32,
                depth_or_array_layers: 1,
            },
        );
    }


    pub fn render(&mut self) -> Result<(), String> {
        let output = self.surface.get_current_texture().map_err(|e| e.to_string())?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Command Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            #[cfg(feature = "gui")]
            let pipeline = if self.gui.wireframe {
                &self.wireframe_pipeline
            } else {
                &self.render_pipeline
            };
            #[cfg(not(feature = "gui"))]
            let pipeline = &self.render_pipeline;

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.aspect_bind_group, &[]);
            render_pass.set_bind_group(1, &self.density_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.buffers.vertex.buffer.slice(..));
            render_pass.set_index_buffer(
                self.buffers.index.buffer.slice(..),
                self.buffers.index.format,
            );
            
            render_pass.draw_indexed(0..self.buffers.index.num_indices, 0, 0..1);
        }

        #[cfg(feature = "gui")]
        self.gui.render(
            &self.window,
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            self.config.width,
            self.config.height,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
