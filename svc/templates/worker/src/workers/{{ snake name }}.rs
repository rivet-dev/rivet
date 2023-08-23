use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "{{pkg}}-{{name}}")]
async fn worker(ctx: &OperationContext<{{snake pkg}}::msg::{{snake name}}::Message>) -> GlobalResult<()> {
	todo!();

	Ok(())
}
