pub mod adapters;
pub mod export;
pub mod paths;
pub mod process;

pub const DEFAULT_OUTPUT_DIR: &str = "dist/typst";

pub type DynError = Box<dyn std::error::Error>;
