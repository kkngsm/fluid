use std::sync::Arc;
use wgpu::{RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline};
use winit::window::Window;
use crate::{
    buffers::Buffers,
    vertex::{Vertex, INDICES},
};

/// wgpuの全コンポーネントを保持し、レンダリングフローを制御するメイン構造体
pub struct State {
    /// 描画対象となるウィンドウの表面（ウィンドウシステムとの橋渡し役）
    surface: wgpu::Surface<'static>,
    /// GPUとのメイン接続。リソース（バッファ、テクスチャ、パイプライン等）の作成に使用する
    device: wgpu::Device,
    /// GPUにコマンドを送り、非同期に実行させるためのキュー
    queue: wgpu::Queue,
    /// ウィンドウサイズや色形式など、サーフェスの詳細な設定
    config: wgpu::SurfaceConfiguration,
    /// 現在のウィンドウの物理サイズ（ピクセル単位）
    pub size: winit::dpi::PhysicalSize<u32>,
    /// 頂点データとフラグメントデータをどう処理するかという一連の描画設定（パイプライン）
    render_pipeline: RenderPipeline,
    /// ウィンドウへのハンドル
    pub window: Arc<Window>,

    /// 頂点バッファやインデックスバッファなどのリソース管理
    buffers: Buffers,

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
                    required_features: wgpu::Features::empty(),
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
        let buffers = Buffers::new(&device);

        #[cfg(feature = "gui")]
        let gui = crate::gui::Gui::new(&window, &device, config.format);

        let bind_group_and_layout = buffers
            .bind_groups
            .iter()
            .map(|bind_group| bind_group.group_and_layout(&device))
            .collect::<Vec<_>>();
        let ( _bind_groups, bind_group_layouts): (Vec<_>, Vec<_>) = bind_group_and_layout
            .iter()
            .map(|(group, layout)| (group, layout))
            .unzip();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &bind_group_layouts.iter().map(|&l| l).collect::<Vec<_>>(),
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
                cull_mode: Some(wgpu::Face::Back),
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

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            window,
            buffers,
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
        }
    }

    pub fn input(&mut self, _: &winit::event::WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
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
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.buffers.vertex.buffer.slice(..));
            render_pass.set_index_buffer(
                self.buffers.index.buffer.slice(..),
                self.buffers.index.format,
            );
            
            // 元々のポリゴンを描画
            render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
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
