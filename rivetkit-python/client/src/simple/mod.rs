use pyo3::prelude::*;

mod sync;
mod r#async;

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    sync::init_module(m)?;
    r#async::init_module(m)?;


    Ok(())
}