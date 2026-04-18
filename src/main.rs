#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    fluid_euler::run().await;
}

#[cfg(target_arch = "wasm32")]
fn main() {}
