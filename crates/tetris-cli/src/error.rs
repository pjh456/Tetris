use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)] // Reserved for CLI error propagation (render/input/config failures)
pub enum CliError {
    #[error("config error: {0}")]
    Config(String),
    #[error("render error: {0}")]
    Render(String),
    #[error("input error: {0}")]
    Input(String),
}
