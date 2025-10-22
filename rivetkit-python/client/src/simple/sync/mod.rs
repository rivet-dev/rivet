use pyo3::prelude::*;

mod handle;
mod client;

pub fn init_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<client::Client>()?;


    Ok(())
}