use chirp_workflow::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestInput {
	pub x: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestOutput {
	pub y: i64,
}

#[workflow(Test)]
pub async fn test(ctx: &mut WorkflowCtx, input: &TestInput) -> GlobalResult<TestOutput> {
	ctx.activity(FooInput {}).await?;

	let sig = ctx.listen::<FooBarSignal>().await?;

	Ok(TestOutput { y: input.x + sig.x })
}

#[signal("foo-bar")]
pub struct FooBarSignal {
	pub x: i64,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FooInput {}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct FooOutput {
	ids: Vec<Uuid>,
}

#[activity(Foo)]
async fn foo(ctx: &ActivityCtx, input: &FooInput) -> GlobalResult<FooOutput> {
	let ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT datacenter_id
		FROM db_cluster.datacenters
		",
	)
	.await?
	.into_iter()
	.map(|(id,)| id)
	.collect();

	Ok(FooOutput { ids })
}
