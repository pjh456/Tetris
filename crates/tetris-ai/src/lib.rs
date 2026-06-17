#![forbid(unsafe_code)]

pub mod env;
pub mod obs;
pub mod reward;

use pyo3::prelude::*;

#[pymodule]
fn tetris_ai(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<env::TetrisEnv>()?;
    Ok(())
}
