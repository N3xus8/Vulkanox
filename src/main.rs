mod error;

use app::App;
use error::Result;
use winit::event_loop::EventLoopBuilder;

mod app;
mod camera;
mod index_buffer;
mod instance_buffer;
mod lighting;
mod mesh;
mod shader;
mod vulkan_context;
mod vulkan_device;
mod vulkan_instance;
mod vulkan_renderer;
mod utils;
fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoopBuilder::new().build()?;

    let mut app = App::new(&event_loop)?;

    event_loop
        .run(move |event, window_target| app.process_event::<()>(event, window_target).unwrap())?;

    Ok(())
}
