use std::sync::Arc;
use wgpu::{RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline};
use winit::window::Window;
use crate::{
    buffers::Buffers,
    vertex::{Vertex},
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
    window: Arc<Window>,

    /// 頂点バッファやインデックスバッファなどのリソース管理
    buffers: Buffers,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let size = window.inner_size();

        // Instance は wgpu ライブラリ全体のエントリポイント。
        // ここから GPU との接続（Adapter）や描画先（Surface）を作成する。
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            // OSが対応している最適なグラフィックスAPI（Vulkan, Metal, DX12など）を自動選択。
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            memory_budget_thresholds: Default::default(),
            display: None,
        });

        // ウィンドウに描画するためのサーフェスを作成
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        // Adapter は実際の GPU（またはソフトウェアレンダラー）のハンドル。
        // ハードウェアの特性や制限などを確認できる。
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // 省電力か高パフォーマンスか。デフォルトはバランス型。
                power_preference: wgpu::PowerPreference::default(),
                // 指定したサーフェスをサポートしているGPUを選択。
                compatible_surface: Some(&surface),
                // ソフトウェアレンダラーなどの代替アダプターを使用するか。
                force_fallback_adapter: false,
            })
            .await
            .expect("適切なグラフィックスアダプターが見つかりませんでした。");

        // Device（リソース作成用）と Queue（コマンド実行用）を要求。
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary Device"),
                    // 今回は特別な拡張機能（レイトレーシングなど）は使用しない。
                    required_features: wgpu::Features::empty(),
                    // GPUに対する制限値。WASM向けにWebGl2互換にするなどの調整が可能。
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                    experimental_features: wgpu::ExperimentalFeatures::default(),
                    trace: wgpu::Trace::default(),
                }
            )
            .await
            .expect("GPUデバイスの作成に失敗しました。");
        // サーフェスの設定。ウィンドウサイズとGPUの色形式を同期させる。
        let caps = surface.get_capabilities(&adapter);
        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap_or_else(|| {
                // サイズが0の場合などのフォールバック設定
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

        // 実際にサイズが有効な場合のみ設定。0の場合は将来のresizeで設定される。
        // if config.width > 0 && config.height > 0 {
        //     surface.configure(&device, &config);
        // }

        // シェーダーモジュールをロード（shader.wgsl）
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // バッファ管理クラスを初期化。頂点バッファなどが作成される。
        let buffers = Buffers::new(&device);

        // バインドグループ（シェーダーに渡す追加データ）の準備。
        // 今回のサンプルでは空だが、将来的に定数などを渡す際に使用。
        let bind_group_and_layout = buffers
            .bind_groups
            .iter()
            .map(|bind_group| bind_group.group_and_layout(&device))
            .collect::<Vec<_>>();
        let ( _bind_groups, bind_group_layouts): (Vec<_>, Vec<_>) = bind_group_and_layout
            .iter()
            .map(|(group, layout)| (group, layout))
            .unzip();

        // 描画パイプラインの全体レイアウト設定。
        let layouts: Vec<Option<&wgpu::BindGroupLayout>> = bind_group_layouts.iter().map(|&l| Some(l)).collect();
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &layouts,
                immediate_size: 0,
            });

        // 描画パイプラインを作成。
        // GPUに対して「頂点データの構造はこう」「このシェーダーで描画して」といった指示を一括で行う。
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            // 頂点シェーダーの設定
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                // 頂点バッファのメモリレイアウト（x, y, zや色情報などの並び）
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            // フラグメントシェーダー（色塗り）の設定
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                // 出力先のテクスチャフォーマット
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE), // 既存の色を上書き
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            // プリミティブ（三角形、点、線など）の設定
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 三角形のリストとして描画
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 反時計回りを表とする（標準的）
                cull_mode: Some(wgpu::Face::Back), // 裏面（カメラから見て時計回り）は描画しない
                ..Default::default()
            },
            depth_stencil: None, // 深度テスト（奥行き判定）は今回は行わない
            multisample: wgpu::MultisampleState {
                count: 1, // アンチエイリアス（MSAA）はなし
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
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
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    /// ウィンドウサイズが変更された際の処理。
    /// サーフェスを再構築しないと描画が止まったり崩れたりする。
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            // 新しいサイズでサーフェスを再構成
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, _: &winit::event::WindowEvent) -> bool {
        // 入力イベント（マウスやキーボード）を処理する場合はここを実装
        false
    }

    pub fn update(&mut self) {
        // ロジックの更新（位置計算など）はここで行う
    }

    /// 実際の描画サイクルを実行。
    pub fn render(&mut self) -> Result<(), String> {
        log::debug!("Rendering frame...");
        // 次に表示されるウィンドウの裏側のテクスチャを取得（スワップチェーン）
        let output = self.surface.get_current_texture();
        let texture = match output {
            wgpu::CurrentSurfaceTexture::Success(texture) => {
                log::debug!("Successfully acquired texture.");
                texture
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
                log::warn!("Suboptimal texture acquired.");
                texture
            }
            wgpu::CurrentSurfaceTexture::Timeout => return Err("Timeout getting texture".to_string()),
            wgpu::CurrentSurfaceTexture::Outdated => return Err("Outdated texture".to_string()),
            wgpu::CurrentSurfaceTexture::Lost => return Err("Lost texture".to_string()),
            wgpu::CurrentSurfaceTexture::Occluded => return Err("Occluded texture".to_string()),
            wgpu::CurrentSurfaceTexture::Validation => return Err("Validation error".to_string()),
        };
        // テクスチャへの「ビュー」を作成
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // GPUに送るための一連のコマンドを組み立てるためのエンコーダーを作成。
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Command Encoder"),
            });

        {
            // レンダリングパス（特定の描画作業のひとかたまり）を開始。
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                // どのテクスチャ（view）に色を描くか。
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // 描画開始時に背景を「黒色」で塗りつぶす
                        load: wgpu::LoadOp::Clear(wgpu::Color::RED), // 【デバッグ】黒ではなく赤で塗りつぶして確認
                        // 描画終了後に結果をGPUメモリに保存する。
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // さきほど作成したパイプラインを「このパスで使う設定」としてセット。
            render_pass.set_pipeline(&self.render_pipeline);
            // 頂点バッファをスロット0にセット。
            render_pass.set_vertex_buffer(0, self.buffers.vertex.buffer.slice(..));
            // インデックスバッファ（描画順序）をセット。
            render_pass.set_index_buffer(
                self.buffers.index.buffer.slice(..),
                self.buffers.index.format,
            );
            // セットされたバッファとインデックスに基づき、三角形を描画。
            // render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
        }

        // コマンドエンコーダーを終了して「コマンドバッファ」にし、キューに投げてGPUに実行させる。
        self.queue.submit(std::iter::once(encoder.finish()));
        // 描画が終わったテクスチャを画面（ウィンドウ）に表示。
        texture.present();
        log::debug!("Frame rendered.");

        Ok(())
    }
}

