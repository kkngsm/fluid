use std::sync::Arc;
use wgpu::{RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline};
use winit::window::Window;
use crate::{
    buffers::{Buffers, BindGroup, BindGroupEntry},
    vertex::Vertex,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AspectRatio {
    ratio: f32,
}

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
    
    vertices: Vec<Vertex>,
    frame_count: u32,
    aspect_bind_group: wgpu::BindGroup,
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

        // 格子状の頂点を生成
        let (vertices, indices) = crate::vertex::create_grid(50, 50);

        // アスペクト比のバインドグループ作成
        let initial_aspect = AspectRatio { ratio: size.width as f32 / size.height as f32 };
        let aspect_entry = BindGroupEntry::uniform(&device, initial_aspect);
        let aspect_bg_def = BindGroup::new("Aspect Bind Group").insert(aspect_entry);
        let aspect_layout = aspect_bg_def.bind_group_layout(&device);
        let aspect_bind_group = aspect_bg_def.bind_group(&device, &aspect_layout);
        
        let mut buffers = Buffers::new(&device, &vertices, &indices);
        
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&aspect_layout],
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
            vertices,
            frame_count: 0,
            aspect_bind_group,
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
            self.buffers.bind_groups[0].entries[0].update(&self.queue, aspect);
        }
    }

    pub fn input(&mut self, _: &winit::event::WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.frame_count += 1;
        let t = self.frame_count as f32 * 0.02;

        // CPU側で頂点カラーを更新
        for v in self.vertices.iter_mut() {
            let x = v.position[0];
            let y = v.position[1];
            
            // 位置と時間に基づいたダイナミックな色計算
            v.color[0] = (x + t).sin() * 0.5 + 0.5;
            v.color[1] = (y + t * 0.5).cos() * 0.5 + 0.5;
            v.color[2] = (x + y + t * 0.7).sin() * 0.5 + 0.5;
        }

        // 更新した頂点データをGPUに送る
        self.buffers.vertex.update(&self.queue, &self.vertices);
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
