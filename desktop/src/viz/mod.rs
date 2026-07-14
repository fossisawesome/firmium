pub mod config;
pub(crate) mod particles;
pub(crate) mod pipeline;
pub(crate) mod shader;
pub mod state;

pub use config::VizConfig;
pub use shader::ShaderVisualizer as Visualizer;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VizMode {
    Bars,
    Lines,
    Scope,
}

impl VizMode {
    pub fn label(self) -> &'static str {
        match self {
            VizMode::Bars => "Bars",
            VizMode::Lines => "Lines",
            VizMode::Scope => "Scope",
        }
    }
}
