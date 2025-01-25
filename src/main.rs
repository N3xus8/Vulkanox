mod error;

use app::App;
use error::Result;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoopBuilder,
};

mod app;
mod vulkan_device;
mod vulkan_instance;
mod vulkan_renderer;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let event_loop = EventLoopBuilder::new().build()?;

    let mut app = App::new(&event_loop)?;

    let mut is_app_started = false;

    event_loop.run(move |event, window_target| match event {
        Event::WindowEvent { window_id, event } => match event {
            WindowEvent::CloseRequested => window_target.exit(),
            _ => {}
        },

        Event::Resumed => {
            if is_app_started {
                app.resume(window_target).unwrap();

            } else {
                
                is_app_started = true;

                app.start(window_target).unwrap();
            }
        }

        Event::Suspended => {
            app.suspend();
        }
        _ => {}
    })?;

    Ok(())
}
