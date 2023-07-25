use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;
use std::collections::HashMap;
use std::convert::TryInto;

#[worker(name = "module-instance-destroy")]
async fn worker(
	ctx: OperationContext<game::msg::instance_destroy::Message>,
) -> Result<(), GlobalError> {
	todo!();

	// TODO: Delete app
	// TODO: Update database

	Ok(())
}
