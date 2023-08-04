use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "{{pkg}}-{{name}}")]
pub async fn handle(
	ctx: OperationContext<{{snake pkg}}::{{snake name}}::Request>,
) -> GlobalResult<{{snake pkg}}::{{snake name}}::Response> {
	todo!();

	// Ok({{snake pkg}}::{{snake name}}::Response {

	// })
}
