use chirp_workflow::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestInput {
	pub x: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestOutput {
	pub y: usize,
}

#[workflow(Test)]
async fn test(ctx: &mut WorkflowCtx, input: &TestInput) -> Result<TestOutput> {
	tracing::info!("input {}", input.x);

	let a = ctx.activity(FooInput {}).await?;

	Ok(TestOutput { y: a.ids.len() })
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct FooInput {}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct FooOutput {
	ids: Vec<Uuid>,
}

#[activity(Foo)]
pub fn foo(ctx: &mut ActivityCtx, input: &FooInput) -> Result<FooOutput> {
	let ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT datacenter_id
		FROM db_cluster.datacenters
		",
	)
	.await
	.unwrap()
	.into_iter()
	.map(|(id,)| id)
	.collect();

	let user_id = util::uuid::parse("000b3124-91d9-472e-8104-3dcc41e1a74d").unwrap();
	let user_get_res = op!([ctx] user_get {
		user_ids: vec![user_id.into()],
	})
	.await
	.unwrap();
	let user = user_get_res.users.first().unwrap();

	tracing::info!(?user, "-----------");

	Ok(FooOutput { ids })
}
