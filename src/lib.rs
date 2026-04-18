pub mod buffers;
pub mod state;
pub mod vertex;

use state::State;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{WindowEvent, ElementState, KeyEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{WindowAttributes, WindowId},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

struct App {
    state: Arc<async_lock::RwLock<Option<State>>>,
}

impl App {
    fn new() -> Self {
        Self {
            state: Arc::new(async_lock::RwLock::new(None)),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let state_lock = self.state.clone();
        
        let is_none = {
            #[cfg(target_arch = "wasm32")]
            {
                state_lock.try_read().map(|guard| guard.is_none()).unwrap_or(false)
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                pollster::block_on(state_lock.read()).is_none()
            }
        };
        
        if is_none {
            let attributes = WindowAttributes::default().with_title("wgpu Fluid Euler");

            #[cfg(target_arch = "wasm32")]
            let attributes = {
                use wasm_bindgen::JsCast;
                use winit::platform::web::WindowAttributesExtWebSys;

                let canvas = web_sys::window()
                    .and_then(|win| win.document())
                    .and_then(|doc| doc.get_element_by_id("canvas"))
                    .and_then(|elem| elem.dyn_into::<web_sys::HtmlCanvasElement>().ok());

                attributes.with_canvas(canvas)
            };

            let window = Arc::new(event_loop.create_window(attributes).unwrap());

            let state_lock = self.state.clone();
            #[cfg(target_arch = "wasm32")]
            {
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    log::info!("Starting State initialization...");
                    let mut state = State::new(window.clone()).await;
                    
                    // ウィンドウから実際のサイズを再取得
                    let size = window.inner_size();
                    log::info!("Actual window size: {:?}", size);
                    state.resize(size);
                    
                    *state_lock.write().await = Some(state);
                    log::info!("State stored in lock.");
                    window.request_redraw();
                    log::info!("Redraw requested.");
                });
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let state = pollster::block_on(State::new(window));
                *pollster::block_on(state_lock.write()) = Some(state);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        let mut guard = match self.state.try_write() {
            Some(g) => g,
            None => return,
        };
        #[cfg(not(target_arch = "wasm32"))]
        let mut guard = pollster::block_on(self.state.write());

        let state = match guard.as_mut() {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),

            WindowEvent::Resized(size) => {
                state.resize(size);
            }

            WindowEvent::RedrawRequested => {
                log::info!("Redraw triggered. Current size: {:?}", state.size);
                if state.size.width > 0 && state.size.height > 0 {
                    match state.render() {
                        Ok(_) => log::info!("Render successful."),
                        Err(e) => {
                            log::error!("Render error: {}", e);
                            // Lost などのエラー時はリサイズを試みる
                            if e.contains("Lost") {
                                state.resize(state.size);
                            } else if e.contains("OutOfMemory") {
                                event_loop.exit();
                            }
                        }
                    }
                } else {
                    log::warn!("Render skipped due to 0 size.");
                }
                state.window().request_redraw();
            }
            _ => {}
        }
    }
}

/// ブラウザから呼び出されるエントリポイント。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::builder().build().unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    let mut app = App::new();
    #[cfg(target_arch = "wasm32")]
    let app = App::new();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        event_loop.run_app(&mut app).unwrap();
    }
}
