
use rivetkit_client::{self as rivetkit_rs};
use pyo3::{prelude::*, types::PyTuple};

use crate::util;

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
        let method = method.to_string();
        let handle = self.handle.clone();
        
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let args = Python::with_gil(|py| util::py_to_json_value(py, &args))?;
            let result = handle.action(&method, args).await;
            let Ok(result) = result else {
                return Err(py_runtime_err!(
                    "Failed to call action: {:?}",
                    result.err()
                ));
            };
            let mut result = Python::with_gil(|py| {
                match util::json_to_py_value(py, &vec![result]) {
                    Ok(value) => Ok(
                        value.iter()
                            .map(|x| x.clone().unbind())
                            .collect::<Vec<PyObject>>()
                    ),
                    Err(e) => Err(e),
                }
            })?;
            let Some(result) = result.drain(0..1).next() else {
                return Err(py_runtime_err!(
                    "Expected one result, got {}",
                    result.len()
                ));
            };

            Ok(result)
        })
    }

    pub fn on_event<'a>(
        &self,
        py: Python<'a>,
        event_name: &str,
        callback: PyObject
    ) -> PyResult<Bound<'a, PyAny>>  {
        let event_name = event_name.to_string();
        let handle = self.handle.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            handle.on_event(&event_name, move |args| {
                if let Err(e) = Python::with_gil(|py| -> PyResult<()> {
                    let args = util::json_to_py_value(py, args)?;
                    let args = PyTuple::new(py, args)?;

                    callback.call(py, args, None)?;

                    Ok(())
                }) {
                    eprintln!("Failed to call event callback: {}", e);
                }
            }).await;

            Ok(())
        })
    }

    pub fn disconnect<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        let handle = self.handle.clone();

        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            handle.disconnect().await;

            Ok(())
        })
    }
}
