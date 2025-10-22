use anyhow::Result;

use super::{DriverConnectArgs, DriverConnection};

pub(crate) async fn connect(_args: DriverConnectArgs) -> Result<DriverConnection> {
    // SSE transport is not currently supported with the new gateway architecture
    // TODO: Implement SSE support via gateway
    Err(anyhow::anyhow!(
        "SSE transport not yet supported with gateway architecture"
    ))
}
