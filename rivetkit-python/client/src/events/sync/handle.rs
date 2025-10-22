use rivetkit_client::{self as rivetkit_rs};
use pyo3::{prelude::*, types::PyTuple};

use crate::util::{self, SYNC_RUNTIME};

#[pyclass]
pub struct ActorHandle {
    pub handle: rivetkit_rs::connection::ActorHandle,
}

#[pymethods]
impl ActorHandle {
    #[new]
    pub fn new() -> PyResult<Self> {
        Err(py_runtime_err!(
            "Actor handle cannot be instantiated directly",
        ))
    }

    pub fn action<'a>(
        &self,
        py: Python<'a>, 
        method: &str,
        args: Vec<PyObject>
    ) -> PyResult<Bound<'a, PyAny>> {
        let result = self.handle.action(
            method,
            util::py_to_json_value(py, &args)?
        );
        let result = SYNC_RUNTIME.block_on(result);
        let Ok(result) = result else {
            return Err(py_runtime_err!(
                "Failed to call action: {:?}",
                result.err()
            ));
        };
        let mut result = util::json_to_py_value(py, &vec![result])?;
        let Some(result) = result.drain(0..1).next() else {
            return Err(py_runtime_err!(
                "Expected one result, got {}",
                result.len()
            ));
        };
        Ok(result)
    }
    
    pub fn on_event(&self, event_name: &str, callback: PyObject) {
        SYNC_RUNTIME.block_on(
            self.handle.on_event(event_name, move |args| {
                if let Err(e) = Python::with_gil(|py| -> PyResult<()> {
                    let args = util::json_to_py_value(py, args)?;
                    let args = PyTuple::new(py, args)?;

                    callback.call(py, args, None)?;

                    Ok(())
                }) {
                    eprintln!("Failed to call event callback: {}", e);
                }
            })
        );
    }

    pub fn disconnect(&self) {
        SYNC_RUNTIME.block_on(
            self.handle.disconnect()
        )
    }
}
