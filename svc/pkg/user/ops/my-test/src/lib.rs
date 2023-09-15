use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "user-my-test")]
pub async fn handle(
	ctx: OperationContext<user::my_test::Request>,
) -> GlobalResult<user::my_test::Response> {
	todo!();

	// Ok(user::my_test::Response {

	// })
}
