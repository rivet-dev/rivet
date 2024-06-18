use chirp_workflow::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestInput {
	pub x: i64,
}

type TestOutput = Result<TestOutputOk, TestOutputErr>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TestOutputOk {
	pub y: usize,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct TestOutputErr {
	pub z: usize,
}

#[workflow(Test)]
pub async fn test(ctx: &mut WorkflowCtx, input: &TestInput) -> GlobalResult<TestOutput> {
	let a = ctx.activity(FooInput {}).await?;

	Ok(Ok(TestOutputOk { y: a.ids.len() }))
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

	let user_id = util::uuid::parse("000b3124-91d9-472e-8104-3dcc41e1a74d")?;
	let user_get_res = op!([ctx] user_get {
		user_ids: vec![user_id.into()],
	})
	.await?;
	let user = unwrap!(user_get_res.users.first());

	Ok(FooOutput { ids })
}
