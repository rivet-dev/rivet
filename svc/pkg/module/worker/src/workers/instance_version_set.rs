use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "module-instance-version-set")]
async fn worker(
	ctx: OperationContext<module::msg::instance_version_set::Message>,
) -> Result<(), GlobalError> {
	todo!();

	Ok(())
}
