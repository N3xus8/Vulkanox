#[allow(unused)]
use vulkano::instance::debug::DebugUtilsLabel;

#[allow(unused)]
macro_rules! debug_label {
    ($label_name:expr) => {
        DebugUtilsLabel {
            label_name: $label_name.to_string(),
            color: [0., 1., 0., 1.], // Green
            ..Default::default()
        }
    };
}   
    #[allow(unused_imports)]
    pub(crate) use debug_label ;
