use thiserror::Error;
use winit::error::OsError;

pub type Result<T> = core::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error>; // for early dev.

#[derive(Error, Debug)]
pub enum VisualSystemError {
    #[error("error creating new VisualSystem: {0}", self)]
    ErrorCreatingVisualSystem,
    #[error("error resizing VisualSystem: {0}", self)]
    ErrorResizingVisualSystem,
    #[error("error drawing VisualSystem: {0}", self)]
    ErrorDrawingVisualSystem,
    #[error("error resuming VisualSystem: {0}", self)]
    ErrorResumingVisualSystem,
    #[error("error request redraw VisualSystem: {0}", self)]
    ErrorRequestReDrawVisualSystem,
    #[error("error creating new Vulkan instance: {0}", self)]
    ErrorCreatingVulkanInstance,
    #[error("error creating new Vulkan device: {0}", self)]
    ErrorCreatingVulkanDevice,
    #[error("error creating new Vulkan renderer: {0}", self)]
    ErrorCreatingVulkanRenderer,

    // -- Externals
    #[error("os error")]
    Os(#[from] OsError),
}
