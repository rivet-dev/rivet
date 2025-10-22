use rivetkit_client::{self as rivetkit_rs, CreateOptions, GetOptions, GetWithIdOptions};
use pyo3::prelude::*;

use super::handle::ActorHandle;
use crate::util::{try_opts_from_kwds, PyKwdArgs, SYNC_RUNTIME};

#[pyclass(name = "Client")]
pub struct Client {
    client: rivetkit_rs::Client,
}

#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature=(
        endpoint,
        transport_kind="websocket",
        encoding_kind="json"
    ))]
    fn py_new(
        endpoint: &str,
        transport_kind: &str,
        encoding_kind: &str,
    ) -> PyResult<Self> {
        let transport_kind = try_transport_kind_from_str(&transport_kind)?;
        let encoding_kind = try_encoding_kind_from_str(&encoding_kind)?;
        let client = rivetkit_rs::Client::new(
            endpoint.to_string(),
            transport_kind,
            encoding_kind
        );

        Ok(Client {
            client
        })
    }

    #[pyo3(signature = (name, **kwds))]
    fn get(&self, name: &str, kwds: Option<PyKwdArgs>) -> PyResult<ActorHandle> {
        let opts = try_opts_from_kwds::<GetOptions>(kwds)?;
        let handle = self.client.get(name, opts);
        let handle = SYNC_RUNTIME.block_on(handle);

        match handle {
            Ok(handle) => Ok(ActorHandle { handle }),
            Err(e) => Err(py_runtime_err!(
                "Failed to get actor: {}",
                e
            ))
        }
    }

    #[pyo3(signature = (id, **kwds))]
    fn get_with_id(&self, id: &str, kwds: Option<PyKwdArgs>) -> PyResult<ActorHandle> {
        let opts = try_opts_from_kwds::<GetWithIdOptions>(kwds)?;
        let handle = self.client.get_with_id(id, opts);
        let handle = SYNC_RUNTIME.block_on(handle);

        match handle {
            Ok(handle) => Ok(ActorHandle { handle }),
            Err(e) => Err(py_runtime_err!(
                "Failed to get actor: {}",
                e
            ))
        }
    }

    #[pyo3(signature = (name, **kwds))]
    fn create(&self, name: &str, kwds: Option<PyKwdArgs>) -> PyResult<ActorHandle> {
        let opts = try_opts_from_kwds::<CreateOptions>(kwds)?;
        let handle = self.client.create(name, opts);
        let handle = SYNC_RUNTIME.block_on(handle);

        match handle {
            Ok(handle) => Ok(ActorHandle { handle }),
            Err(e) => Err(py_runtime_err!(
                "Failed to get actor: {}",
                e
            ))
        }
    }
}

fn try_transport_kind_from_str(
    transport_kind: &str
) -> PyResult<rivetkit_rs::TransportKind> {
    match transport_kind {
        "websocket" => Ok(rivetkit_rs::TransportKind::WebSocket),
        "sse" => Ok(rivetkit_rs::TransportKind::Sse),
        _ => Err(py_value_err!(
            "Invalid transport kind: {}",
            transport_kind
        )),
    }
}

fn try_encoding_kind_from_str(
    encoding_kind: &str
) -> PyResult<rivetkit_rs::EncodingKind> {
    match encoding_kind {
        "json" => Ok(rivetkit_rs::EncodingKind::Json),
        "cbor" => Ok(rivetkit_rs::EncodingKind::Cbor),
        _ => Err(py_value_err!(
            "Invalid encoding kind: {}",
            encoding_kind
        )),
    }
}
