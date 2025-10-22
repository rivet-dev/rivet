use rivetkit_client::{
    client, CreateOptions, GetOptions, GetWithIdOptions
};
use once_cell::sync::Lazy;
use pyo3::{prelude::*, types::PyDict};
use tokio::runtime::{self, Runtime};

pub static SYNC_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap()
});

macro_rules! py_runtime_err {
    ($msg:expr) => {
        pyo3::exceptions::PyRuntimeError::new_err($msg)
    };
    ($msg:expr, $($arg:tt)*) => {
        pyo3::exceptions::PyRuntimeError::new_err(format!(
            $msg,
            $($arg)*
        ))
    };
}

macro_rules! py_value_err {
    ($msg:expr) => {
        pyo3::exceptions::PyValueError::new_err($msg)
    };
    ($msg:expr, $($arg:tt)*) => {
        pyo3::exceptions::PyValueError::new_err(format!(
            $msg,
            $($arg)*
        ))
    };
}

// See ACTR-96
pub fn py_to_json_value(
    py: Python<'_>,
    py_obj: &Vec<PyObject>
) -> PyResult<Vec<serde_json::Value>> {
    let py_json = py.import("json")?;
    
    let obj_strs: Vec<String> = py_obj
        .into_iter()
        .map(|obj| {
            let obj_str: String = py_json
                .call_method("dumps", (obj,), None)?
                .extract::<String>()?;

            Ok(obj_str)
        })
        .collect::<PyResult<Vec<String>>>()?;

    let json_value = obj_strs
        .into_iter()
        .map(|s| {
            match serde_json::from_str(&s) {
                Ok(value) => Ok(value),
                Err(e) => Err(py_value_err!(
                    "Failed to parse JSON: {}",
                    e
                ))
            }
        })
        .collect::<PyResult<Vec<serde_json::Value>>>()?;
    
    Ok(json_value)
}

pub fn json_to_py_value<'a>(
    py: Python<'a>,
    val: &Vec<serde_json::Value>
) -> PyResult<Vec<Bound<'a, PyAny>>> {
    let py_json = py.import("json")?;
    
    val.into_iter()
    .map(|f| {
        let str = serde_json::to_string(f)
            .map_err(|e| py_value_err!(
                "Failed to serialize JSON value: {}",
                e
            ))?;
        
        py_json
            .call_method("loads", (str,), None)
            .map_err(|e| py_value_err!(
                "Failed to load JSON value: {}",
                e
            ))
    })
    .collect()
}

fn extract_tags(tags: Option<Bound<'_, PyAny>>) -> PyResult<Option<Vec<(String, String)>>> {
    let Some(tags) = tags else {
        return Ok(None)
    };

    // tags should be a Dict<String, String>
    // Convert it to a Vec<(String, String)>
    let tags_dict = tags.downcast::<PyDict>().map_err(|_| {
        py_value_err!(
            "Invalid tags format. Expected a dictionary with both string keys and values"
        )
    })?;
    let tags = tags_dict
        .iter()
        .map(|(key, value)| {
            let key: String = key.extract()?;
            let value: String = value.extract()?;
            Ok((key, value))
        })
        .collect::<PyResult<Vec<(String, String)>>>()
        .map_err(|_| {
            py_value_err!(
                "Invalid tags format. Expected a dictionary with both string keys and values"
            )
        })?;

    Ok(Some(tags))
}

fn extract_params(params: Option<Bound<'_, PyAny>>) -> PyResult<Option<serde_json::Value>> {
    let Some(params) = params else {
        return Ok(None)
    };

    let value = Python::with_gil(|py| py_to_json_value(py, &vec![params.unbind()]))?;
    let Some(value) = value.first() else {
        return Err(py_runtime_err!("Failed to convert params to JSON value"));
    };

    Ok(Some(value.clone()))
}


pub type PyKwdArgs<'a> = Bound<'a, PyDict>;
pub struct PyKwdArgsWrapper<'a>(pub PyKwdArgs<'a>);
pub fn try_opts_from_kwds<'a, T>(kwds: Option<PyKwdArgs<'a>>) -> PyResult<T>
where
    T: TryFrom<PyKwdArgsWrapper<'a>, Error = PyErr> + Default,
{
    let opts = kwds.map_or(Ok(T::default()), |kwds| {
        T::try_from(PyKwdArgsWrapper(kwds))
    })?;

    Ok(opts)
}

impl TryFrom::<PyKwdArgsWrapper<'_>> for GetOptions {
    type Error = PyErr;

    fn try_from(kwds: PyKwdArgsWrapper) -> PyResult<Self> {
        let tags = extract_tags(kwds.0.get_item("tags")?)?;

        let params = match kwds.0.get_item("params")? {
            Some(params) => {
                let value = Python::with_gil(|py| py_to_json_value(py, &vec![params.unbind()]))?;
                let Some(value) = value.first() else {
                    return Err(py_runtime_err!("Failed to convert params to JSON value"));
                };

                Some(value.clone())
            },
            None => None
        };

        let no_create = match kwds.0.get_item("no_create")? {
            Some(no_create) => {
                Some(no_create.extract::<bool>().map_err(|_| {
                    py_value_err!(
                        "Invalid no_create format. Expected a boolean"
                    )
                })?)
            },
            None => None
        };

        let create = match kwds.0.get_item("create")? {
            Some(create) => {
                let create_req_metadata = create.downcast::<PyDict>().map_err(|_| {
                    py_value_err!(
                        "Invalid create format. Expected a dictionary"
                    )
                })?;

                let tags = extract_tags(create_req_metadata.get_item("tags")?)?;

                let region =  create_req_metadata
                    .get_item("region")?
                    .map(|v| v.extract::<String>())
                    .transpose()?;

                Some(client::PartialCreateRequestMetadata {
                    tags,
                    region
                })
            },
            None => None
        };

        Ok(GetOptions {
            tags,
            params,
            no_create,
            create
        })
    }
}

impl TryFrom::<PyKwdArgsWrapper<'_>> for CreateOptions {
    type Error = PyErr;

    fn try_from(kwds: PyKwdArgsWrapper) -> PyResult<Self> {
        let params = extract_params(kwds.0.get_item("params")?)?;

        let create = match kwds.0.get_item("create")? {
            Some(create) => {
                let create_req_metadata = create.downcast::<PyDict>().map_err(|_| {
                    py_value_err!(
                        "Invalid create format. Expected a dictionary"
                    )
                })?;

                let tags = extract_tags(create_req_metadata.get_item("tags")?)?
                    .unwrap_or_default();

                let region =  create_req_metadata
                    .get_item("region")?
                    .map(|v| v.extract::<String>())
                    .transpose()?;

                client::CreateRequestMetadata {
                    tags,
                    region
                }
            },
            None => client::CreateRequestMetadata::default()
        };

        Ok(CreateOptions {
            params,
            create,
        })
    }
}

impl TryFrom::<PyKwdArgsWrapper<'_>> for GetWithIdOptions {
    type Error = PyErr;

    fn try_from(kwds: PyKwdArgsWrapper) -> PyResult<Self> {
        let params = extract_params(kwds.0.get_item("params")?)?;

        Ok(GetWithIdOptions {
            params
        })
    }
}
