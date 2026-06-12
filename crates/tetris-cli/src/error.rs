use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("config error: {0}")]
    Config(String),
    #[error("render error: {0}")]
    Render(String),
    #[error("input error: {0}")]
    Input(String),
}
