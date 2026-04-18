#![cfg(feature = "gui")]

use egui_winit::State as EguiState;

use egui_wgpu::Renderer as EguiRenderer;

use winit::window::Window;

use wgpu::{Device, Queue, TextureFormat, CommandEncoder, TextureView, RenderPass};


pub struct Gui {
    pub egui_state: EguiState,
    pub egui_renderer: EguiRenderer,
    pub checkbox_state: bool,
    pub wireframe: bool,
}

#[cfg(feature = "gui")]
impl Gui {
    pub fn new(window: &Window, device: &Device, config_format: TextureFormat) -> Self {
        let egui_context = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(device, config_format, None, 1, false);

        Self {
            egui_state,
            egui_renderer,
            checkbox_state: false,
            wireframe: false,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        let response = self.egui_state.on_window_event(window, event);
        response.consumed
    }

    pub fn render(
        &mut self,
        window: &Window,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        config_width: u32,
        config_height: u32,
    ) {
        let input = self.egui_state.take_egui_input(window);
        self.egui_state.egui_ctx().begin_pass(input);

        // UIの定義
        egui::Window::new("Settings").show(self.egui_state.egui_ctx(), |ui| {
            ui.checkbox(&mut self.checkbox_state, "Check me!");
            ui.checkbox(&mut self.wireframe, "Wireframe");
        });


        let full_output = self.egui_state.egui_ctx().end_pass();
        let paint_jobs = self.egui_state.egui_ctx().tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config_width, config_height],
            pixels_per_point: window.scale_factor() as f32,
        };

        // テクスチャの更新
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // egui の描画コマンド作成
        self.egui_renderer.update_buffers(
            device,
            queue,
            encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            unsafe {
                let render_pass_static: &mut RenderPass<'static> = std::mem::transmute(&mut render_pass);
                self.egui_renderer.render(render_pass_static, &paint_jobs, &screen_descriptor);
            }
        }
    }
}
